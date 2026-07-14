use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginType, UpdateAuthority,
    },
};

use crate::constants::*;
use crate::error::ErrorCode;
use crate::macros::ai;
use crate::state::Config;

#[derive(Accounts)]
pub struct Stake<'info> {
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
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ ErrorCode::InvalidUpdateAuthority
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(
        mut,
       has_one = update_authority @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub collection: Account<'info, BaseCollectionV1>,

    /// CHECK: we verify it is the update authority tied to collection
    #[account(
        seeds = [SEED_UPDATE_AUTHORITY, collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, crate::MplCore>,
}

pub fn handler(ctx: Context<Stake>) -> Result<()> {
    // fetch existing attributes
    let attributes_fetched: Option<Attributes> =
        fetch_plugin::<BaseAssetV1, Attributes>(&ai!(ctx, asset), PluginType::Attributes)
            .ok()
            .map(|(_, attrs, _)| attrs);

    // prepare the attributes list to add or update based on the existing attrs
    let mut attributes_list: Vec<Attribute> = Vec::new();

    // loop thru all attrs and save ! staked and staked_at
    if let Some(attributes) = &attributes_fetched {
        for attribute in &attributes.attribute_list {
            if attribute.key == "staked" {
                require!(attribute.value == "false", ErrorCode::AlreadyStaked);
            } else if attribute.key != "staked_at" {
                attributes_list.push(attribute.clone());
            }
        }
    }

    // Add the staking attributes
    attributes_list.push(Attribute {
        key: "staked".to_string(),
        value: "true".to_string(),
    });
    attributes_list.push(Attribute {
        key: "staked_at".to_string(),
        value: Clock::get()?.unix_timestamp.to_string(),
    });

    // list of attributes resolved
    // add or update the existing plugin
    // the attributes plugin is an authority managed pllugin so it needs to be
    // signed/update by the update authority (PDA of program)

    // prepare signing seeds for the update auth
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds: &[&[u8]; 3] = &[
        SEED_UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];
    let mpl_core_program = ai!(ctx, mpl_core_program);
    let asset = ai!(ctx, asset);
    let collection = ai!(ctx, collection);
    let owner = ai!(ctx, owner);
    let authority = ai!(ctx, update_authority);
    let system_program = ai!(ctx, system_program);

    // add if the attributes plugin does not exist
    if attributes_fetched.is_none() {
        AddPluginV1CpiBuilder::new(&mpl_core_program)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&owner)
            .authority(Some(&authority))
            .system_program(&system_program)
            .plugin(Plugin::Attributes(Attributes {
                attribute_list: attributes_list,
            }))
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke_signed(&[signer_seeds])?;
    }
    // we found it .. update
    else {
        UpdatePluginV1CpiBuilder::new(&mpl_core_program)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&owner)
            .authority(Some(&authority))
            .system_program(&system_program)
            .plugin(Plugin::Attributes(Attributes {
                attribute_list: attributes_list,
            }))
            .invoke_signed(&[signer_seeds])?;
    }

    let freeze_delegate: Option<FreezeDelegate> =
        fetch_plugin::<BaseAssetV1, FreezeDelegate>(&asset, PluginType::FreezeDelegate)
            .ok()
            .map(|(_, d, _)| d);

    if freeze_delegate.is_none() {
        // Freeze the asset.
        //
        // FreezeDelegate is an owner-managed plugin: adding it must be approved by
        // the asset OWNER, who is a real transaction signer, so this is a plain
        // `invoke` with the owner as authority (unlike the Attributes CPIs above,
        // which the program signs as the update authority via `invoke_signed`).
        // `init_authority` then hands the new plugin to the update-authority PDA,
        // which is what lets unstake thaw it with `invoke_signed` later.

        AddPluginV1CpiBuilder::new(&mpl_core_program)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&owner)
            .authority(Some(&owner))
            .system_program(&system_program)
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            // hand over authority to our PDA
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke()?;
    } else {
        UpdatePluginV1CpiBuilder::new(&mpl_core_program)
            .asset(&asset)
            .collection(Some(&collection))
            .payer(&owner)
            .authority(Some(&authority))
            .system_program(&system_program)
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .invoke_signed(&[signer_seeds])?;
    }

    Ok(())
}
