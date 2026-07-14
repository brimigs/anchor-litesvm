use crate::state::PollAccount;
use anchor_lang::prelude::*;

const SEED_POLL: &[u8] = b"poll";

#[derive(Accounts)]
#[instruction(poll_id: u64, start: u64, end: u64, name: String, description: String)]
pub struct InitializePoll<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 8 + PollAccount::INIT_SPACE,
        seeds = [SEED_POLL, poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll_account: Account<'info, PollAccount>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializePoll<'info> {
    pub fn initialize_poll(
        &mut self,
        _poll_id: u64,
        start: u64,
        end: u64,
        name: String,
        description: String,
    ) -> Result<()> {
        self.poll_account.set_inner(PollAccount {
            poll_name: name,
            poll_description: description,
            poll_voting_start: start,
            poll_voting_end: end,
            poll_option_index: 0,
        });
        Ok(())
    }
}
