use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to_checked, Mint, MintToChecked, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginType, UpdateAuthority},
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::macros::ai;
use crate::state::Config;

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
       seeds = [SEED_CONFIG, collection.key().as_ref()],
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

    /// CHECK: why this is safe
    #[account(
        seeds = [SEED_UPDATE_AUTHORITY, collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_REWARDS_MINT, config.key().as_ref()],
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

pub fn handler(ctx: Context<Unstake>) -> Result<()> {
    // fetch existing attributes
    let attributes_fetched =
        fetch_plugin::<BaseAssetV1, Attributes>(&ai!(ctx, asset), PluginType::Attributes)
            .ok()
            .map(|(_, attrs, _)| attrs);

    require!(attributes_fetched.is_some(), ErrorCode::AssetNotStaked);

    let attributes = attributes_fetched.unwrap();

    // prepare list to update based on existing attrs

    let mut attributes_list: Vec<Attribute> =
        Vec::with_capacity(attributes.attribute_list.len() + 1);

    // additional aux vars
    let current_timestamp: i64 = Clock::get()?.unix_timestamp;
    let mut staked_timestamp: i64 = 0;
    // Days already paid out by claim_rewards are tracked in last_claimed; an
    // asset that was never claimed has no such attribute, which reads as 0.
    let mut last_claimed: i64 = 0;

    for attribute in &attributes.attribute_list {
        if attribute.key == "staked" {
            require!(attribute.value == "true", ErrorCode::AssetNotStaked);
        } else if attribute.key == "staked_at" {
            staked_timestamp = attribute
                .value
                .parse::<i64>()
                .map_err(|_| ErrorCode::InvalidTimestamp)?;
        } else if attribute.key == "last_claimed" {
            last_claimed = attribute
                .value
                .parse::<i64>()
                .map_err(|_| ErrorCode::InvalidTimestamp)?;
        } else {
            attributes_list.push(attribute.clone());
        }
    }

    // The freeze gate measures from the original stake; what has already been
    // claimed has no bearing on when the asset may exit.
    let staked_time = current_timestamp
        .checked_sub(staked_timestamp)
        .ok_or(ErrorCode::InvalidTimestamp)?
        .checked_div(SECONDS_PER_DAY)
        .ok_or(ErrorCode::InvalidTimestamp)?;
    require!(
        staked_time >= ctx.accounts.config.freeze_period as i64,
        ErrorCode::FreezePeriodNotElapsed
    );

    // The payout starts where claim_rewards left off (or at the stake, if
    // nothing was ever claimed), so claim-then-unstake never double-pays.
    let unpaid_days = current_timestamp
        .checked_sub(staked_timestamp.max(last_claimed))
        .ok_or(ErrorCode::InvalidTimestamp)?
        .checked_div(SECONDS_PER_DAY)
        .ok_or(ErrorCode::InvalidTimestamp)?;

    // prepare signing seeds for update auth
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds: &[&[u8]; 3] = &[
        SEED_UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    // update the asset attributes plugin with existing attrs, including
    // staking attributes with updated values

    // add staking attributes first (reset values)
    attributes_list.push(Attribute {
        key: "staked".to_string(),
        value: "false".to_string(),
    });
    attributes_list.push(Attribute {
        key: "staked_at".to_string(),
        value: "0".to_string(),
    });
    attributes_list.push(Attribute {
        key: "last_claimed".to_string(),
        value: "0".to_string(),
    });

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

    UpdatePluginV1CpiBuilder::new(&ai!(ctx, mpl_core_program))
        .asset(&ai!(ctx, asset))
        .collection(Some(&ai!(ctx, collection)))
        .payer(&ai!(ctx, owner))
        .authority(Some(&ai!(ctx, update_authority)))
        .system_program(&ai!(ctx, system_program))
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
        .invoke_signed(&[signer_seeds])?;

    // mint rewards to the user

    // calculate the amount
    let amount: u64 = (unpaid_days as u64)
        .checked_mul(ctx.accounts.config.rewards_bps as u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_mul(10u64.pow(ctx.accounts.rewards_mint.decimals as u32))
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_div(10000u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?;

    //  prepare signer seeds for config PDA
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
