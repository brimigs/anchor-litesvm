//! Version anti-corruption boundary for the litesvm harness-runtime seam.
//!
//! Everything the workspace needs from litesvm and its solana-runtime crates is
//! re-exported here, so a solana/litesvm version bump edits this crate and not
//! the call sites across `litesvm-utils` and `anchor-litesvm`. See
//! `NOTES/2026-07-13-compat-anti-corruption-layer.md` for the boundary contract.
//!
//! The anchor and on-chain program contract (`anchor_lang`, `solana_program`,
//! `spl_*`) is out of scope and flows through directly. `Pubkey` in particular
//! stays on `solana_program` (the anchor-pinned side) rather than here, so it
//! never splits from the type anchor programs speak.

// Runtime engine and transaction machinery.
pub use litesvm::types::TransactionMetadata;
pub use litesvm::LiteSVM;
pub use solana_account::Account;
pub use solana_hash::Hash;
pub use solana_message::Message;
pub use solana_signature::Signature;
pub use solana_transaction::{versioned::VersionedTransaction, Transaction};

// Client and test-side vocabulary that crosses the seam.
pub use solana_keypair::Keypair;
pub use solana_signer::Signer;

// CPI-invocation-tree rendering (LiteSVM/litesvm#349).
pub use litesvm_cpi_tree::{cpi_tree, format_cpi_tree, CpiFrame, CpiOutcome, CpiTreeExt, FrameLog};

#[cfg(test)]
mod tests {
    use super::{Keypair, LiteSVM, Signer};

    #[test]
    fn facade_types_are_usable() {
        // Exercise the re-exported vocab and engine handle at runtime, not just
        // their paths: the `pub use` lines above already fail to compile if any
        // path is wrong, so a version bump that moves a type is caught there.
        // A keypair's pubkey is the canonical 32 bytes.
        let payer = Keypair::new();
        assert_eq!(payer.pubkey().as_ref().len(), 32);

        // The engine handle constructs through the boundary.
        let _svm = LiteSVM::new();
    }
}
