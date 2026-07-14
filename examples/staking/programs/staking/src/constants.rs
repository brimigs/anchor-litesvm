use anchor_lang::prelude::*;

#[constant]
pub const SECONDS_PER_DAY: i64 = 86_400;

#[constant]
pub const SEED_UPDATE_AUTHORITY: &[u8] = b"update_authority";

#[constant]
pub const SEED_CONFIG: &[u8] = b"config";

#[constant]
pub const SEED_REWARDS_MINT: &[u8] = b"rewards_mint";
