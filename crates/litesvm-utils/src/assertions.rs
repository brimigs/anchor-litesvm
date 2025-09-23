//! Assertion helpers for testing account states
//!
//! This module provides convenient assertion methods for verifying
//! account states in tests.

use litesvm::LiteSVM;
use solana_program::pubkey::Pubkey;
use litesvm_token::spl_token;
use solana_program_pack::Pack;

/// Assertion helper methods for LiteSVM
pub trait AssertionHelpers {
    /// Assert that an account is closed (doesn't exist or has 0 lamports and 0 data)
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::AssertionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let account = Pubkey::new_unique();
    /// svm.assert_account_closed(&account);
    /// ```
    fn assert_account_closed(&self, pubkey: &Pubkey);

    /// Assert that an account exists
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::AssertionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let account = Pubkey::new_unique();
    /// svm.assert_account_exists(&account);
    /// ```
    fn assert_account_exists(&self, pubkey: &Pubkey);

    /// Assert token account balance
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::AssertionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let token_account = Pubkey::new_unique();
    /// svm.assert_token_balance(&token_account, 1_000_000_000); // 1 token with 9 decimals
    /// ```
    fn assert_token_balance(&self, token_account: &Pubkey, expected: u64);

    /// Assert SOL balance
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::AssertionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let account = Pubkey::new_unique();
    /// svm.assert_sol_balance(&account, 1_000_000_000); // 1 SOL
    /// ```
    fn assert_sol_balance(&self, pubkey: &Pubkey, expected: u64);

    /// Assert token mint supply
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::AssertionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let mint = Pubkey::new_unique();
    /// svm.assert_mint_supply(&mint, 1_000_000_000);
    /// ```
    fn assert_mint_supply(&self, mint: &Pubkey, expected: u64);

    /// Assert that an account is owned by a specific program
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::AssertionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let account = Pubkey::new_unique();
    /// # let owner = Pubkey::new_unique();
    /// svm.assert_account_owner(&account, &owner);
    /// ```
    fn assert_account_owner(&self, account: &Pubkey, expected_owner: &Pubkey);

    /// Assert that an account has a specific data length
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::AssertionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let account = Pubkey::new_unique();
    /// svm.assert_account_data_len(&account, 100);
    /// ```
    fn assert_account_data_len(&self, account: &Pubkey, expected_len: usize);
}

impl AssertionHelpers for LiteSVM {
    fn assert_account_closed(&self, pubkey: &Pubkey) {
        let account = self.get_account(pubkey);
        assert!(
            account.is_none() || (account.as_ref().unwrap().lamports == 0 && account.as_ref().unwrap().data.is_empty()),
            "Expected account {} to be closed, but it exists with {} lamports and {} bytes of data",
            pubkey,
            account.as_ref().map_or(0, |a| a.lamports),
            account.as_ref().map_or(0, |a| a.data.len())
        );
    }

    fn assert_account_exists(&self, pubkey: &Pubkey) {
        let account = self.get_account(pubkey);
        assert!(
            account.is_some(),
            "Expected account {} to exist, but it doesn't",
            pubkey
        );
    }

    fn assert_token_balance(&self, token_account: &Pubkey, expected: u64) {
        let account = self
            .get_account(token_account)
            .expect(&format!("Token account {} not found", token_account));

        let token_data = spl_token::state::Account::unpack(&account.data)
            .expect(&format!("Failed to unpack token account {}", token_account));

        assert_eq!(
            token_data.amount, expected,
            "Token balance mismatch for account {}. Expected: {}, Actual: {}",
            token_account, expected, token_data.amount
        );
    }

    fn assert_sol_balance(&self, pubkey: &Pubkey, expected: u64) {
        let account = self.get_account(pubkey);
        let actual = account.map_or(0, |a| a.lamports);
        assert_eq!(
            actual, expected,
            "SOL balance mismatch for account {}. Expected: {}, Actual: {}",
            pubkey, expected, actual
        );
    }

    fn assert_mint_supply(&self, mint: &Pubkey, expected: u64) {
        let account = self
            .get_account(mint)
            .expect(&format!("Mint {} not found", mint));

        let mint_data = spl_token::state::Mint::unpack(&account.data)
            .expect(&format!("Failed to unpack mint {}", mint));

        assert_eq!(
            mint_data.supply, expected,
            "Mint supply mismatch for {}. Expected: {}, Actual: {}",
            mint, expected, mint_data.supply
        );
    }

    fn assert_account_owner(&self, account: &Pubkey, expected_owner: &Pubkey) {
        let acc = self
            .get_account(account)
            .expect(&format!("Account {} not found", account));

        assert_eq!(
            &acc.owner, expected_owner,
            "Account owner mismatch for {}. Expected: {}, Actual: {}",
            account, expected_owner, acc.owner
        );
    }

    fn assert_account_data_len(&self, account: &Pubkey, expected_len: usize) {
        let acc = self
            .get_account(account)
            .expect(&format!("Account {} not found", account));

        assert_eq!(
            acc.data.len(),
            expected_len,
            "Account data length mismatch for {}. Expected: {}, Actual: {}",
            account,
            expected_len,
            acc.data.len()
        );
    }
}