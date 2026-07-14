use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub rewards_bps: u16,   // rewardss pct in basis points (why so large?)
    pub freeze_period: u16, // min freeze period in days
    pub rewards_bump: u8,   // bump for the rewards mint acct
    pub bump: u8,           // bump for the config acct
}
