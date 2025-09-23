//! # anchor-litesvm
//!
//! A comprehensive testing utility library that bridges Anchor and LiteSVM,
//! reducing test code by up to 70% while maintaining full control.
//!
//! ## Features
//!
//! - **Fluent Instruction Builder**: Build and execute instructions with chainable API
//! - **Transaction Helpers**: One-line transaction execution with automatic error handling
//! - **Test Account Helpers**: Create funded accounts, mints, and token accounts in single calls
//! - **Assertion Helpers**: Clean, readable test assertions for account states
//! - **Type-Safe Deserialization**: Automatic Anchor account unpacking with proper types
//! - **Direct LiteSVM Access**: Full control when you need it
//!
//! ## Quick Start
//!
//! ```ignore
//! use anchor_litesvm::{AnchorLiteSVM, TestHelpers, AssertionHelpers, tuple_args};
//! use solana_program::pubkey::Pubkey;
//!
//! // Initialize with one line!
//! let program_id = Pubkey::new_unique();
//! let program_bytes = include_bytes!("../target/deploy/program.so");
//! let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
//!
//! // Create test accounts in one line each
//! let maker = ctx.create_funded_account(10_000_000_000).unwrap();
//! let mint = ctx.create_token_mint(&maker, 9).unwrap();
//!
//! // Build and execute instructions with fluent API
//! let result = ctx.instruction_builder("transfer")
//!     .signer("from", &maker)
//!     .account_mut("to", Pubkey::new_unique())
//!     .args(tuple_args((100u64,)))  // No struct needed!
//!     .execute(&mut ctx, &[&maker])
//!     .unwrap();
//!
//! // Clean assertions
//! result.assert_success();
//! ctx.assert_token_balance(&mint.pubkey(), 100);
//! ```

pub mod account;
pub mod assertions;
pub mod builder;
pub mod context;
pub mod instruction;
pub mod instruction_builder;
pub mod test_helpers;
pub mod transaction;

// Re-export main types for convenience
pub use account::{get_anchor_account, get_anchor_account_unchecked, AccountError};
pub use assertions::AssertionHelpers;
pub use builder::{AnchorLiteSVM, ProgramTestExt};
pub use context::AnchorContext;
pub use instruction::{build_anchor_instruction, calculate_anchor_discriminator};
pub use instruction_builder::{InstructionBuilder, tuple_args, TupleArgs};
pub use test_helpers::TestHelpers;
pub use transaction::{TransactionError, TransactionHelpers, TransactionResult};

// Re-export commonly used external types
pub use litesvm::LiteSVM;
pub use solana_program::instruction::{AccountMeta, Instruction};
pub use solana_program::pubkey::Pubkey;

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
        let ctx = AnchorContext::new(svm, program_id);

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

        let instruction = ctx
            .build_instruction("test", accounts, TestArgs { value: 42 })
            .unwrap();

        assert_eq!(instruction.program_id, program_id);
        assert!(!instruction.data.is_empty());
    }
}