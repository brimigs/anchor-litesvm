pub use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account(discriminator = 1)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub expiry: i64, // clock::UnixTimestamp,
    pub receive: u64,
    pub bump: u8,
}

pub const EXPIRY_PERIOD_IN_SECONDS: i64 = 90 * 24 * 60 * 60;
