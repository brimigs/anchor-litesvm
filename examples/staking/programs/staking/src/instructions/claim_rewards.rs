use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to_checked, Mint, MintToChecked, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginType, UpdateAuthority},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::macros::ai;
use crate::state::Config;

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        has_one = owner @ ErrorCode::InvalidOwner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(
        mut,
        has_one = update_authority @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub collection: Account<'info, BaseCollectionV1>,

    /// CHECK: seeds-verified PDA; signs the mpl-core attribute-update CPI as
    /// the collection's update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"rewards_mint", config.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub mpl_core_program: Program<'info, crate::MplCore>,
}

pub fn handler(ctx: Context<ClaimRewards>) -> Result<()> {
    // Read the staking attributes; an asset with none was never staked.
    let attributes_fetched: Option<Attributes> =
        fetch_plugin::<BaseAssetV1, Attributes>(&ai!(ctx, asset), PluginType::Attributes)
            .ok()
            .map(|(_, attrs, _)| attrs);
    require!(attributes_fetched.is_some(), ErrorCode::AssetNotStaked);
    let attributes = attributes_fetched.unwrap();

    let now: i64 = Clock::get()?.unix_timestamp;

    // Walk the attribute list once: pull out the staking bookkeeping
    // (staked / staked_at / last_claimed), preserve everything else untouched.
    // A missing last_claimed (asset staked but never claimed) reads as 0.
    let mut staked = false;
    let mut staked_at: i64 = 0;
    let mut last_claimed: i64 = 0;
    let mut attributes_list: Vec<Attribute> =
        Vec::with_capacity(attributes.attribute_list.len() + 1);

    for attribute in &attributes.attribute_list {
        match attribute.key.as_str() {
            "staked" => {
                require!(attribute.value == "true", ErrorCode::AssetNotStaked);
                staked = true;
            }
            "staked_at" => {
                staked_at = attribute
                    .value
                    .parse::<i64>()
                    .map_err(|_| ErrorCode::InvalidTimestamp)?;
            }
            "last_claimed" => {
                last_claimed = attribute
                    .value
                    .parse::<i64>()
                    .map_err(|_| ErrorCode::InvalidTimestamp)?;
            }
            _ => attributes_list.push(attribute.clone()),
        }
    }
    require!(staked, ErrorCode::AssetNotStaked);

    // The freeze gate measures from the original stake, exactly like unstake.
    // Claiming never moves staked_at, so once the gate opens it stays open.
    let staked_days = now
        .checked_sub(staked_at)
        .ok_or(ErrorCode::InvalidTimestamp)?
        .checked_div(SECONDS_PER_DAY)
        .ok_or(ErrorCode::InvalidTimestamp)?;
    require!(
        staked_days >= ctx.accounts.config.freeze_period as i64,
        ErrorCode::FreezePeriodNotElapsed
    );

    // Accrual runs from the later of stake / previous claim. Pay whole days
    // only, and advance last_claimed by exactly the days paid (not to `now`),
    // so partial days carry into the next claim instead of rounding away.
    let accrual_start = staked_at.max(last_claimed);
    let accrued_days = now
        .checked_sub(accrual_start)
        .ok_or(ErrorCode::InvalidTimestamp)?
        .checked_div(SECONDS_PER_DAY)
        .ok_or(ErrorCode::InvalidTimestamp)?;
    require!(accrued_days > 0, ErrorCode::NothingToClaim);

    let new_last_claimed = accrual_start
        .checked_add(
            accrued_days
                .checked_mul(SECONDS_PER_DAY)
                .ok_or(ErrorCode::InvalidTimestamp)?,
        )
        .ok_or(ErrorCode::InvalidTimestamp)?;

    // Rebuild the attribute list: preserved extras + the staking bookkeeping,
    // with only last_claimed changed.
    attributes_list.push(Attribute {
        key: "staked".to_string(),
        value: "true".to_string(),
    });
    attributes_list.push(Attribute {
        key: "staked_at".to_string(),
        value: staked_at.to_string(),
    });
    attributes_list.push(Attribute {
        key: "last_claimed".to_string(),
        value: new_last_claimed.to_string(),
    });

    // Write the updated attributes. The Attributes plugin is
    // authority-managed, so the update-authority PDA signs.
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds: &[&[u8]; 3] = &[
        SEED_UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    UpdatePluginV1CpiBuilder::new(&ai!(ctx, mpl_core_program))
        .asset(&ai!(ctx, asset))
        .collection(Some(&ai!(ctx, collection)))
        .payer(&ai!(ctx, owner))
        .authority(Some(&ai!(ctx, update_authority)))
        .system_program(&ai!(ctx, system_program))
        .plugin(Plugin::Attributes(Attributes {
            attribute_list: attributes_list,
        }))
        .invoke_signed(&[signer_seeds])?;

    // Mint the rewards, signed by the config PDA (the mint authority).
    // Same math as unstake: days * bps * 10^decimals / 10_000.
    let amount: u64 = (accrued_days as u64)
        .checked_mul(ctx.accounts.config.rewards_bps as u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_mul(10u64.pow(ctx.accounts.rewards_mint.decimals as u32))
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_div(10_000u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?;

    let config_seeds: &[&[u8]; 3] = &[
        b"config",
        collection_key.as_ref(),
        &[ctx.accounts.config.bump],
    ];
    let config_signer_seeds: &[&[&[u8]]; 1] = &[&config_seeds[..]];

    mint_to_checked(
        CpiContext::new_with_signer(
            ai!(ctx, token_program),
            MintToChecked {
                mint: ai!(ctx, rewards_mint),
                to: ai!(ctx, user_rewards_ata),
                authority: ai!(ctx, config),
            },
            config_signer_seeds,
        ),
        amount,
        ctx.accounts.rewards_mint.decimals,
    )?;

    Ok(())
}
