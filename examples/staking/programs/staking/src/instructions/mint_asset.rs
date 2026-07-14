use anchor_lang::prelude::*;
use mpl_core::{accounts::BaseCollectionV1, instructions::CreateV2CpiBuilder};

use crate::constants::*;
use crate::macros::ai;

#[derive(Accounts)]
pub struct MintAsset<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub asset: Signer<'info>,

    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    /// CHECK: why this is safe
    #[account(
        seeds = [SEED_UPDATE_AUTHORITY, collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, crate::MplCore>,
}

pub fn handler(ctx: Context<MintAsset>, name: String, uri: String) -> Result<()> {
    // signer seeds for the update authority
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds: &[&[u8]] = &[
        SEED_UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    CreateV2CpiBuilder::new(&ai!(ctx, mpl_core_program))
        .asset(&ai!(ctx, asset))
        .collection(Some(&ai!(ctx, collection)))
        .authority(Some(&ai!(ctx, update_authority)))
        .payer(&ai!(ctx, user))
        .owner(Some(&ai!(ctx, user)))
        .update_authority(None)
        .system_program(&ai!(ctx, system_program))
        .name(name)
        .uri(uri)
        .invoke_signed(&[signer_seeds])?;

    Ok(())
}
