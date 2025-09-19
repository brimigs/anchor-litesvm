//! Assertion helpers for testing account states
//!
//! This module provides convenient assertion methods for verifying
//! account states in tests.

use crate::AnchorContext;
use solana_program::pubkey::Pubkey;
use litesvm_token::spl_token;
use solana_program_pack::Pack;

/// Assertion helper methods for AnchorContext
pub trait AssertionHelpers {
    /// Assert that an account is closed (doesn't exist or has 0 lamports and 0 data)
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, AssertionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let account = Pubkey::new_unique();
    /// ctx.assert_account_closed(&account);
    /// ```
    fn assert_account_closed(&self, pubkey: &Pubkey);

    /// Assert that an account exists
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, AssertionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let account = Pubkey::new_unique();
    /// ctx.assert_account_exists(&account);
    /// ```
    fn assert_account_exists(&self, pubkey: &Pubkey);

    /// Assert token account balance
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, AssertionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let token_account = Pubkey::new_unique();
    /// ctx.assert_token_balance(&token_account, 1_000_000_000); // 1 token with 9 decimals
    /// ```
    fn assert_token_balance(&self, token_account: &Pubkey, expected: u64);

    /// Assert token account balance with custom message
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, AssertionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let token_account = Pubkey::new_unique();
    /// ctx.assert_token_balance_with_msg(
    ///     &token_account,
    ///     1_000_000_000,
    ///     "Maker should have 1 token"
    /// );
    /// ```
    fn assert_token_balance_with_msg(&self, token_account: &Pubkey, expected: u64, msg: &str);

    /// Assert account lamports balance
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, AssertionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let account = Pubkey::new_unique();
    /// ctx.assert_account_lamports(&account, 1_000_000_000);
    /// ```
    fn assert_account_lamports(&self, pubkey: &Pubkey, expected: u64);

    /// Assert account owner
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, AssertionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let account = Pubkey::new_unique();
    /// # let expected_owner = Pubkey::new_unique();
    /// ctx.assert_account_owner(&account, &expected_owner);
    /// ```
    fn assert_account_owner(&self, pubkey: &Pubkey, expected_owner: &Pubkey);

    /// Assert that multiple accounts are closed
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, AssertionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let account1 = Pubkey::new_unique();
    /// # let account2 = Pubkey::new_unique();
    /// ctx.assert_accounts_closed(&[&account1, &account2]);
    /// ```
    fn assert_accounts_closed(&self, pubkeys: &[&Pubkey]);
}

impl AssertionHelpers for AnchorContext {
    fn assert_account_closed(&self, pubkey: &Pubkey) {
        match self.svm.get_account(pubkey) {
            None => {
                // Account doesn't exist - that's fine, it's closed
            }
            Some(account) => {
                // In LiteSVM, closed accounts might still exist with 0 lamports and empty data
                assert!(
                    account.lamports == 0 && account.data.is_empty(),
                    "Account {} should be closed but has {} lamports and {} bytes of data",
                    pubkey,
                    account.lamports,
                    account.data.len()
                );
            }
        }
    }

    fn assert_account_exists(&self, pubkey: &Pubkey) {
        assert!(
            self.svm.get_account(pubkey).is_some(),
            "Account {} should exist but was not found",
            pubkey
        );
    }

    fn assert_token_balance(&self, token_account: &Pubkey, expected: u64) {
        self.assert_token_balance_with_msg(
            token_account,
            expected,
            &format!("Token balance mismatch for {}", token_account),
        );
    }

    fn assert_token_balance_with_msg(&self, token_account: &Pubkey, expected: u64, msg: &str) {
        match self.svm.get_account(token_account) {
            None => {
                if expected == 0 {
                    // Account doesn't exist and we expect 0 balance - that's fine
                    return;
                }
                panic!(
                    "{}: Account {} doesn't exist but expected {} tokens",
                    msg, token_account, expected
                );
            }
            Some(account) => {
                let token_state = spl_token::state::Account::unpack(&account.data)
                    .expect(&format!("Failed to unpack token account {}", token_account));

                assert_eq!(
                    token_state.amount, expected,
                    "{}: expected {} tokens, got {}",
                    msg, expected, token_state.amount
                );
            }
        }
    }

    fn assert_account_lamports(&self, pubkey: &Pubkey, expected: u64) {
        let account = self
            .svm
            .get_account(pubkey)
            .expect(&format!("Account {} should exist", pubkey));

        assert_eq!(
            account.lamports, expected,
            "Account {} lamports mismatch: expected {}, got {}",
            pubkey, expected, account.lamports
        );
    }

    fn assert_account_owner(&self, pubkey: &Pubkey, expected_owner: &Pubkey) {
        let account = self
            .svm
            .get_account(pubkey)
            .expect(&format!("Account {} should exist", pubkey));

        assert_eq!(
            account.owner, *expected_owner,
            "Account {} owner mismatch: expected {}, got {}",
            pubkey, expected_owner, account.owner
        );
    }

    fn assert_accounts_closed(&self, pubkeys: &[&Pubkey]) {
        for pubkey in pubkeys {
            self.assert_account_closed(pubkey);
        }
    }
}

/// Additional assertion functions that don't require self
pub mod assertions {
    use super::*;

    /// Assert that two pubkeys are equal with a custom message
    pub fn assert_pubkey_eq(actual: &Pubkey, expected: &Pubkey, msg: &str) {
        assert_eq!(
            actual, expected,
            "{}: expected {}, got {}",
            msg, expected, actual
        );
    }

    /// Assert that a value is within a range
    pub fn assert_in_range(value: u64, min: u64, max: u64, msg: &str) {
        assert!(
            value >= min && value <= max,
            "{}: value {} is not in range [{}, {}]",
            msg,
            value,
            min,
            max
        );
    }

    /// Assert that a token amount matches expected value within some tolerance
    /// Useful for dealing with rounding in token calculations
    pub fn assert_token_amount_approx(actual: u64, expected: u64, tolerance: u64, msg: &str) {
        let diff = if actual > expected {
            actual - expected
        } else {
            expected - actual
        };

        assert!(
            diff <= tolerance,
            "{}: expected {} ± {}, got {} (diff: {})",
            msg,
            expected,
            tolerance,
            actual,
            diff
        );
    }
}