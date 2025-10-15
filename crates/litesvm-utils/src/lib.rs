//! # litesvm-utils
//!
//! Framework-agnostic testing utilities for LiteSVM that dramatically simplify Solana program testing.
//!
//! This crate provides essential helpers that work with **any** Solana program (not just Anchor):
//! - Account creation and funding (one-liners)
//! - Token operations (mints, accounts, minting)
//! - Transaction execution with rich result analysis
//! - Assertion helpers for testing account states
//! - PDA derivation utilities
//! - Clock and slot manipulation
//!
//! ## Why litesvm-utils?
//!
//! **Before (Raw LiteSVM):**
//! ```rust,ignore
//! // 30+ lines to create a token mint
//! let mint = Keypair::new();
//! let rent = svm.minimum_balance_for_rent_exemption(82);
//! let create_account_ix = system_instruction::create_account(/*...*/);
//! let init_mint_ix = spl_token::instruction::initialize_mint(/*...*/);
//! // ... transaction building, signing, sending ...
//! ```
//!
//! **After (litesvm-utils):**
//! ```rust,ignore
//! let mint = svm.create_token_mint(&authority, 9)?; // One line
//! ```
//!
//! ## Features
//!
//! ### Test Account Helpers
//! Create funded accounts, mints, and token accounts in single calls:
//! ```rust,ignore
//! let user = svm.create_funded_account(10_000_000_000)?;
//! let accounts = svm.create_funded_accounts(5, 1_000_000_000)?;
//! ```
//!
//! ### Token Operations
//! One-line token operations without manual transaction building:
//! ```rust,ignore
//! let mint = svm.create_token_mint(&authority, 9)?;
//! let token_account = svm.create_associated_token_account(&mint.pubkey(), &owner)?;
//! svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000)?;
//! ```
//!
//! ### Transaction Helpers
//! Execute transactions with automatic result analysis:
//! ```rust,ignore
//! let result = svm.send_instruction(ix, &[&signer])?;
//! result.assert_success();
//! assert!(result.compute_units() < 200_000);
//! ```
//!
//! ### Assertion Helpers
//! Clean, readable test assertions:
//! ```rust,ignore
//! svm.assert_token_balance(&token_account, 1_000_000);
//! svm.assert_sol_balance(&user.pubkey(), 10_000_000_000);
//! svm.assert_account_exists(&pda);
//! svm.assert_account_closed(&closed_account);
//! ```
//!
//! ### PDA Utilities
//! Convenient PDA derivation:
//! ```rust,ignore
//! let pda = svm.get_pda(&[b"vault", user.pubkey().as_ref()], &program_id);
//! let (pda, bump) = svm.get_pda_with_bump(&[b"seed"], &program_id);
//! ```
//!
//! ### Clock Manipulation
//! Test time-based logic:
//! ```rust,ignore
//! let slot = svm.get_current_slot();
//! svm.advance_slot(100);
//! ```
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use litesvm_utils::{LiteSVMBuilder, TestHelpers, AssertionHelpers, TransactionHelpers};
//! use solana_program::pubkey::Pubkey;
//!
//! // 1. Initialize with one line
//! let program_id = Pubkey::new_unique();
//! let program_bytes = include_bytes!("../target/deploy/program.so");
//! let mut svm = LiteSVMBuilder::build_with_program(program_id, program_bytes);
//!
//! // 2. Create test accounts in one line each
//! let maker = svm.create_funded_account(10_000_000_000).unwrap();
//! let taker = svm.create_funded_account(10_000_000_000).unwrap();
//!
//! // 3. Create token infrastructure
//! let mint = svm.create_token_mint(&maker, 9).unwrap();
//! let maker_ata = svm.create_associated_token_account(&mint.pubkey(), &maker).unwrap();
//! svm.mint_to(&mint.pubkey(), &maker_ata, &maker, 1_000_000_000).unwrap();
//!
//! // 4. Execute instruction and analyze results
//! let result = svm.send_instruction(ix, &[&maker]).unwrap();
//! result.assert_success();
//! assert!(result.has_log("Transfer complete"));
//!
//! // 5. Verify with clean assertions
//! svm.assert_token_balance(&maker_ata, 1_000_000_000);
//! svm.assert_sol_balance(&maker.pubkey(), 10_000_000_000);
//! ```
//!
//! ## Complete Example
//!
//! ```rust,ignore
//! use litesvm_utils::{LiteSVMBuilder, TestHelpers, AssertionHelpers, TransactionHelpers};
//!
//! #[test]
//! fn test_token_transfer() {
//!     // Setup
//!     let mut svm = LiteSVMBuilder::build_with_program(program_id, program_bytes);
//!
//!     // Create accounts
//!     let sender = svm.create_funded_account(10_000_000_000).unwrap();
//!     let receiver = svm.create_funded_account(10_000_000_000).unwrap();
//!
//!     // Setup tokens
//!     let mint = svm.create_token_mint(&sender, 9).unwrap();
//!     let sender_ata = svm.create_associated_token_account(&mint.pubkey(), &sender).unwrap();
//!     let receiver_ata = svm.create_associated_token_account(&mint.pubkey(), &receiver).unwrap();
//!     svm.mint_to(&mint.pubkey(), &sender_ata, &sender, 1_000_000).unwrap();
//!
//!     // Execute transfer
//!     let result = svm.send_instruction(transfer_ix, &[&sender]).unwrap();
//!     result.assert_success();
//!
//!     // Verify
//!     svm.assert_token_balance(&sender_ata, 500_000);
//!     svm.assert_token_balance(&receiver_ata, 500_000);
//! }
//! ```
//!
//! ## Framework Agnostic
//!
//! Unlike `anchor-litesvm`, this crate works with **any** Solana program:
//! - Native Solana programs
//! - Anchor programs
//! - Solana Program Library (SPL) programs
//! - Custom frameworks
//!
//! ## Traits
//!
//! - [`TestHelpers`] - Account and token creation helpers
//! - [`AssertionHelpers`] - Test assertion methods
//! - [`TransactionHelpers`] - Transaction execution helpers
//!
//! ## Modules
//!
//! - [`assertions`] - Assertion helper implementations
//! - [`builder`] - Test environment builders
//! - [`test_helpers`] - Test helper implementations
//! - [`transaction`] - Transaction execution and result analysis

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