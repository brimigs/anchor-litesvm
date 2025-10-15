//! # anchor-litesvm
//!
//! Testing framework for Anchor programs using LiteSVM.
//!
//! This crate provides a **simplified syntax similar to anchor-client** but without RPC overhead,
//! achieving **78% code reduction** compared to raw LiteSVM.
//!
//! ## Why anchor-litesvm?
//!
//! | Feature | anchor-client + LiteSVM | anchor-litesvm |
//! |---------|-------------------------|----------------|
//! | **Code Lines** | 279 | **106 (78% less)** |
//! | **Compilation** | Slow (network deps) | **40% faster** |
//! | **Setup** | Mock RPC needed | **One line** |
//! | **Syntax** | anchor-client | **Similar to anchor-client** |
//! | **Helpers** | Manual | **Built-in** |
//!
//! ## Key Features
//!
//! - **Simplified Syntax**: Similar to anchor-client
//! - **No Mock RPC Setup**: One-line initialization
//! - **Integrated Test Helpers**: Token operations, assertions, event parsing
//! - **Familiar API**: If you know anchor-client, you know this
//! - **Transferable Knowledge**: Test skills apply to production
//! - **Type Safety**: Compile-time validation with Anchor types
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use anchor_litesvm::{AnchorLiteSVM, TestHelpers, AssertionHelpers};
//! use solana_sdk::signature::Signer;
//!
//! // 1. Generate client types from your program
//! anchor_lang::declare_program!(my_program);
//!
//! #[test]
//! fn test_my_program() {
//!     // 2. One-line setup (no mock RPC needed)
//!     let mut ctx = AnchorLiteSVM::build_with_program(
//!         my_program::ID,
//!         include_bytes!("../target/deploy/my_program.so"),
//!     );
//!
//!     // 3. Create test accounts with helpers
//!     let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
//!     let mint = ctx.svm.create_token_mint(&user, 9).unwrap();
//!
//!     // 4. Build instruction (simplified syntax - similar to anchor client)
//!     let ix = ctx.program()
//!         .accounts(my_program::client::accounts::Transfer {
//!             from: sender_account,
//!             to: recipient_account,
//!             authority: user.pubkey(),
//!             token_program: spl_token::id(),
//!         })
//!         .args(my_program::client::args::Transfer { amount: 100 })
//!         .instruction()?;
//!
//!     // 5. Execute and verify
//!     ctx.execute_instruction(ix, &[&user])?.assert_success();
//!     ctx.svm.assert_token_balance(&recipient_account, 100);
//! }
//! ```
//!
//! ## Common Patterns
//!
//! ### Token Operations
//!
//! ```rust,ignore
//! use litesvm_utils::TestHelpers;
//!
//! let mint = ctx.svm.create_token_mint(&authority, 9)?;
//! let token_account = ctx.svm.create_associated_token_account(&mint.pubkey(), &owner)?;
//! ctx.svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000)?;
//! ```
//!
//! ### PDA Derivation
//!
//! ```rust,ignore
//! // Just the address
//! let pda = ctx.svm.get_pda(&[b"vault", user.pubkey().as_ref()], &program_id);
//!
//! // With bump seed
//! let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"vault"], &program_id);
//! ```
//!
//! ### Error Testing
//!
//! ```rust,ignore
//! let result = ctx.execute_instruction(ix, &[&user])?;
//! result.assert_failure();
//! result.assert_error("insufficient funds");
//! result.assert_error_code(6000); // Anchor custom error
//! ```
//!
//! ### Event Parsing
//!
//! ```rust,ignore
//! use anchor_litesvm::EventHelpers;
//!
//! let events: Vec<TransferEvent> = result.parse_events()?;
//! result.assert_event_emitted::<TransferEvent>();
//! ```
//!
//! ### Account Deserialization
//!
//! ```rust,ignore
//! let account: MyAccountType = ctx.get_account(&pda)?;
//! assert_eq!(account.authority, user.pubkey());
//! ```
//!
//! ## Documentation
//!
//! - [Quick Start Guide](https://github.com/brimigs/anchor-litesvm/blob/main/docs/QUICK_START.md)
//! - [API Reference](https://github.com/brimigs/anchor-litesvm/blob/main/docs/API_REFERENCE.md)
//! - [Migration Guide](https://github.com/brimigs/anchor-litesvm/blob/main/docs/MIGRATION.md)
//! - [Examples](https://github.com/brimigs/anchor-litesvm/tree/main/examples)
//!
//! ## Modules
//!
//! - [`account`] - Account deserialization utilities
//! - [`builder`] - Test environment builders
//! - [`context`] - Main test context (`AnchorContext`)
//! - [`events`] - Event parsing helpers
//! - [`instruction`] - Instruction building utilities
//! - [`program`] - Simplified Program API

pub mod account;
pub mod builder;
pub mod context;
pub mod events;
pub mod instruction;
pub mod program;

// Re-export main types for convenience
pub use account::{get_anchor_account, get_anchor_account_unchecked, AccountError};
pub use builder::{AnchorLiteSVM, ProgramTestExt};
pub use context::AnchorContext;
pub use events::{parse_event_data, EventError, EventHelpers};
pub use instruction::{build_anchor_instruction, calculate_anchor_discriminator};
pub use program::{InstructionBuilder, Program};

// Re-export litesvm-utils functionality for convenience
pub use litesvm_utils::{
    AssertionHelpers, LiteSVMBuilder, TestHelpers, TransactionError, TransactionHelpers,
    TransactionResult,
};

// Re-export commonly used external types
pub use anchor_lang::{AccountDeserialize, AnchorSerialize};
pub use litesvm::LiteSVM;
pub use solana_program::instruction::{AccountMeta, Instruction};
pub use solana_program::pubkey::Pubkey;
pub use solana_sdk::signature::{Keypair, Signer};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use anchor_lang::AnchorSerialize;
    use borsh::BorshSerialize;

    #[test]
    fn test_full_workflow() {
        // Create test context
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let _ctx = AnchorContext::new(svm, program_id);

        // Test instruction building
        #[derive(BorshSerialize)]
        struct TestArgs {
            value: u64,
        }

        impl AnchorSerialize for TestArgs {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                BorshSerialize::serialize(self, writer)
            }
        }

        let accounts = vec![
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ];

        let instruction = build_anchor_instruction(
            &program_id,
            "test",
            accounts,
            TestArgs { value: 42 },
        )
        .unwrap();

        assert_eq!(instruction.program_id, program_id);
        assert!(!instruction.data.is_empty());
    }
}