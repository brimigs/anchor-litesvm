use crate::{Escrow, ESCROW_SEED, EXPIRY_PERIOD_IN_SECONDS};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer_checked, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
#[instruction(seeds: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account[
        mint::token_program = token_program
    ]]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account[
        mint::token_program = token_program
    ]]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account[
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    ]]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account[
        init,
        payer = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), seeds.to_le_bytes().as_ref()],
        space = Escrow::DISCRIMINATOR.len() + Escrow::INIT_SPACE,
        bump,
    ]]
    pub escrow: Account<'info, Escrow>,

    #[account[
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    ]]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Make<'info> {
    // Initialize Escrow
    pub fn init_escrow(&mut self, seed: u64, receive: u64, bumps: &MakeBumps) -> Result<()> {
        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            expiry: {
                let clock = Clock::get()?;
                let now = clock.unix_timestamp;
                now + EXPIRY_PERIOD_IN_SECONDS // 90 days
            },
            receive,
            bump: bumps.escrow,
        });

        Ok(())
    }

    pub fn deposit(&mut self, deposit: u64) -> Result<()> {
        let transfer_account = TransferChecked {
            from: self.maker_ata_a.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.key(), transfer_account);
        let _ = transfer_checked(cpi_ctx, deposit, self.mint_a.decimals);

        Ok(())
    }
}
