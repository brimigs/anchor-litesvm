use crate::state::{CandidateAccount, PollAccount};
use anchor_lang::prelude::*;

const SEED_POLL: &[u8] = b"poll";

#[derive(Accounts)]
#[instruction(poll_id: u64, candidate: String)]
pub struct InitializeCandidate<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_POLL, poll_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub poll_account: Account<'info, PollAccount>,
    #[account(
        init,
        payer = signer,
        space = 8 + CandidateAccount::INIT_SPACE,
        seeds = [poll_id.to_le_bytes().as_ref(), candidate.as_ref()],
        bump,
    )]
    pub candidate_account: Account<'info, CandidateAccount>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeCandidate<'info> {
    pub fn initialize_candidate(&mut self, _poll_id: u64, candidate: String) -> Result<()> {
        self.candidate_account.candidate_name = candidate;
        self.poll_account.poll_option_index += 1;
        Ok(())
    }
}
