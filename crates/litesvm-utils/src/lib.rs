//! # litesvm-utils
//!
//! General testing utilities for LiteSVM that simplify Solana program testing.
//!
//! This crate provides framework-agnostic helpers for:
//! - Account creation and funding
//! - Token operations (mints, accounts, minting)
//! - Transaction execution and result analysis
//! - Assertion helpers for testing
//! - PDA derivation
//!
//! ## Features
//!
//! - **Test Account Helpers**: Create funded accounts, mints, and token accounts in single calls
//! - **Transaction Helpers**: One-line transaction execution with automatic error handling
//! - **Assertion Helpers**: Clean, readable test assertions for account states
//! - **Builder Pattern**: Fluent API for setting up test environments
//! - **Framework Agnostic**: Works with any Solana program, not just Anchor
//!
//! ## Quick Start
//!
//! ```ignore
//! use litesvm_utils::{LiteSVMBuilder, TestHelpers, AssertionHelpers, TransactionHelpers};
//! use solana_program::pubkey::Pubkey;
//!
//! // Initialize with one line!
//! let program_id = Pubkey::new_unique();
//! let program_bytes = include_bytes!("../target/deploy/program.so");
//! let mut svm = LiteSVMBuilder::build_with_program(program_id, program_bytes);
//!
//! // Create test accounts in one line each
//! let maker = svm.create_funded_account(10_000_000_000).unwrap();
//! let mint = svm.create_token_mint(&maker, 9).unwrap();
//!
//! // Send instructions and analyze results
//! let result = svm.send_instruction(ix, &[&maker]).unwrap();
//! result.assert_success();
//!
//! // Clean assertions
//! svm.assert_token_balance(&token_account, 100);
//! svm.assert_sol_balance(&maker.pubkey(), 10_000_000_000);
//! ```

pub mod assertions;
pub mod builder;
pub mod test_helpers;
pub mod transaction;

// Re-export main types for convenience
pub use assertions::AssertionHelpers;
pub use builder::{LiteSVMBuilder, ProgramTestExt};
pub use test_helpers::TestHelpers;
pub use transaction::{TransactionError, TransactionHelpers, TransactionResult};

// Re-export commonly used external types
pub use litesvm::LiteSVM;
pub use solana_program::pubkey::Pubkey;
pub use solana_sdk::signature::Keypair;