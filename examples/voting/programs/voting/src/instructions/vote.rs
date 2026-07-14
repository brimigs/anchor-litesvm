#[cfg(feature = "guards")]
use crate::error::ErrorCode;
use crate::state::{CandidateAccount, PollAccount, VoteReceiptAccount};
use anchor_lang::prelude::*;

const SEED_POLL: &[u8] = b"poll";
const SEED_VOTE_RECEIPT: &[u8] = b"vote_receipt";

#[derive(Accounts)]
#[instruction(poll_id: u64, candidate: String)]
pub struct Vote<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_POLL, poll_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub poll_account: Account<'info, PollAccount>,
    #[account(
        mut,
        seeds = [poll_id.to_le_bytes().as_ref(), candidate.as_ref()],
        bump,
    )]
    pub candidate_account: Account<'info, CandidateAccount>,
    #[account(
        init,
        payer = signer,
        space = 8 + VoteReceiptAccount::INIT_SPACE,
        seeds = [SEED_VOTE_RECEIPT, poll_id.to_le_bytes().as_ref(), signer.key().as_ref()],
        bump,
    )]
    pub vote_receipt: Account<'info, VoteReceiptAccount>,
    pub system_program: Program<'info, System>,
}

impl<'info> Vote<'info> {
    // One vote per signer per poll: the `init` on `vote_receipt` (seeded by
    // poll_id + signer, no candidate) fails the second call from the same
    // signer, so the duplicate-vote check is structural rather than a runtime
    // guard inside this handler.
    pub fn vote(&mut self, poll_id: u64, _candidate: String) -> Result<()> {
        #[cfg(feature = "guards")]
        {
            let now: i64 = Clock::get()?.unix_timestamp;
            if now > (self.poll_account.poll_voting_end as i64) {
                return Err(ErrorCode::VotingEnded.into());
            }
            if now <= (self.poll_account.poll_voting_start as i64) {
                return Err(ErrorCode::VotingNotStarted.into());
            }
        }

        self.candidate_account.candidate_votes += 1;
        self.vote_receipt.poll_id = poll_id;
        self.vote_receipt.voter = self.signer.key();
        self.vote_receipt.candidate = self.candidate_account.key();
        Ok(())
    }
}
