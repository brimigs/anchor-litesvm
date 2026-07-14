use anchor_lang::prelude::*;
use mpl_core::instructions::CreateCollectionV2CpiBuilder;

use crate::constants::*;
use crate::macros::ai;

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub collection: Signer<'info>,

    /// CHECK: This account is not init and is being used for signing purps only
    /// we verify it derives from the correct seed
    #[account(
        seeds = [SEED_UPDATE_AUTHORITY, collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, crate::MplCore>,
}

pub fn handler(ctx: Context<CreateCollection>, name: String, uri: String) -> Result<()> {
    //signer seeds for the update authority
    let collection_key: Pubkey = ctx.accounts.collection.key();
    let signer_seeds: &[&[u8]; 3] = &[
        SEED_UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    CreateCollectionV2CpiBuilder::new(&ai!(ctx, mpl_core_program))
        .collection(&ai!(ctx, collection))
        .payer(&ai!(ctx, payer))
        .update_authority(Some(&ai!(ctx, update_authority)))
        .system_program(&ai!(ctx, system_program))
        .name(name)
        .uri(uri)
        .invoke_signed(&[signer_seeds])?;

    Ok(())
}
