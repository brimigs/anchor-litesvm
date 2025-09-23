//! # anchor-litesvm
//!
//! Anchor-specific testing utilities for LiteSVM that simplify Anchor program testing.
//!
//! This crate provides Anchor framework integration with LiteSVM, including:
//! - Native instruction building with anchor-client compatible API
//! - Event parsing and assertion utilities
//! - Anchor error handling and testing
//! - CPI (Cross-Program Invocation) testing utilities
//! - Account constraint validation helpers
//! - IDL integration for automatic size calculation
//!
//! ## Features
//!
//! - **Native Instruction Building**: Production-ready syntax without RPC overhead
//! - **Event Testing**: Parse and assert on Anchor events
//! - **Error Handling**: Type-safe Anchor error assertions
//! - **CPI Testing**: Mock and test cross-program invocations
//! - **Constraint Testing**: Validate account constraints
//! - **IDL Support**: Load IDLs and calculate account sizes
//!
//! ## Quick Start
//!
//! ```ignore
//! use anchor_litesvm::{AnchorLiteSVM, tuple_args};
//! use solana_program::pubkey::Pubkey;
//!
//! // Initialize with Anchor program
//! let program_id = Pubkey::new_unique();
//! let program_bytes = include_bytes!("../target/deploy/my_anchor_program.so");
//! let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
//!
//! // Build and execute Anchor instructions with fluent API
//! let result = ctx.instruction_builder("transfer")
//!     .signer("from", &maker)
//!     .account_mut("to", recipient)
//!     .args(tuple_args((100u64,)))  // No struct needed!
//!     .execute(&mut ctx, &[&maker])
//!     .unwrap();
//!
//! // Get Anchor accounts with automatic deserialization
//! let account: MyAnchorAccount = ctx.get_account(&account_pubkey).unwrap();
//! ```

pub mod account;
pub mod builder;
pub mod client;
pub mod context;
pub mod instruction;
pub mod instruction_builder;

// Re-export main types for convenience
pub use account::{get_anchor_account, get_anchor_account_unchecked, AccountError};
pub use builder::{AnchorLiteSVM, ProgramTestExt};
pub use client::{ClientBuilder, LiteSvmClient};
pub use context::AnchorContext;
pub use instruction::{build_anchor_instruction, calculate_anchor_discriminator};
#[allow(deprecated)]
pub use instruction_builder::{InstructionBuilder, tuple_args, TupleArgs};

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