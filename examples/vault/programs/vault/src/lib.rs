// clippy::diverging_sub_expression fires inside Anchor's #[program] macro
// expansion (a known false positive on the macro-generated error paths).
// The lint pierces module-level #[allow], so we set it at the crate root.
#![allow(clippy::diverging_sub_expression)]

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("6RviLVy2WPGm7QYfCuZq66vKWF58WVTNWfFE7RgWxcfP");

/// A deposit landed: who, how much, and the vault's balance after.
#[event]
pub struct Deposited {
    pub user: Pubkey,
    pub amount: u64,
    pub vault_balance: u64,
}

#[program]
pub mod vault {
    use super::*;

    // Initialize program and accounts
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    // depositing funds to that vault
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    // withdraw funds
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    // close vault
    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()
    }
}
