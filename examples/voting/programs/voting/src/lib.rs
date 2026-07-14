use anchor_lang::prelude::*;

pub mod error;
pub mod state;

#[cfg(feature = "poll")]
pub mod instructions;
#[cfg(feature = "poll")]
pub use instructions::*;

pub use state::*;

declare_id!("GdPDj9mvShPP3EvnF8FZzRcLxJKxgQG7R3qAWr5R1tZU");

#[program]
pub mod voting {
    use super::*;

    #[cfg(feature = "poll")]
    pub fn initialize_poll(
        ctx: Context<InitializePoll>,
        poll_id: u64,
        start: u64,
        end: u64,
        name: String,
        description: String,
    ) -> Result<()> {
        ctx.accounts
            .initialize_poll(poll_id, start, end, name, description)
    }

    #[cfg(feature = "candidate")]
    pub fn initialize_candidate(
        ctx: Context<InitializeCandidate>,
        poll_id: u64,
        candidate: String,
    ) -> Result<()> {
        ctx.accounts.initialize_candidate(poll_id, candidate)
    }

    #[cfg(feature = "vote")]
    pub fn vote(ctx: Context<Vote>, poll_id: u64, candidate: String) -> Result<()> {
        ctx.accounts.vote(poll_id, candidate)
    }
}
