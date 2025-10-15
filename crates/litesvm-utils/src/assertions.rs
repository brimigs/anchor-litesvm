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
            .unwrap_or_else(|| panic!("Token account {} not found", token_account));

        let token_data = spl_token::state::Account::unpack(&account.data)
            .unwrap_or_else(|_| panic!("Failed to unpack token account {}", token_account));

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
            .unwrap_or_else(|| panic!("Mint {} not found", mint));

        let mint_data = spl_token::state::Mint::unpack(&account.data)
            .unwrap_or_else(|_| panic!("Failed to unpack mint {}", mint));

        assert_eq!(
            mint_data.supply, expected,
            "Mint supply mismatch for {}. Expected: {}, Actual: {}",
            mint, expected, mint_data.supply
        );
    }

    fn assert_account_owner(&self, account: &Pubkey, expected_owner: &Pubkey) {
        let acc = self
            .get_account(account)
            .unwrap_or_else(|| panic!("Account {} not found", account));

        assert_eq!(
            &acc.owner, expected_owner,
            "Account owner mismatch for {}. Expected: {}, Actual: {}",
            account, expected_owner, acc.owner
        );
    }

    fn assert_account_data_len(&self, account: &Pubkey, expected_len: usize) {
        let acc = self
            .get_account(account)
            .unwrap_or_else(|| panic!("Account {} not found", account));

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestHelpers;
    use solana_sdk::signature::Signer;

    #[test]
    fn test_assert_account_closed_nonexistent() {
        let svm = LiteSVM::new();
        let nonexistent_account = Pubkey::new_unique();

        // Should not panic for non-existent account
        svm.assert_account_closed(&nonexistent_account);
    }

    #[test]
    fn test_assert_account_exists() {
        let mut svm = LiteSVM::new();
        let account = svm.create_funded_account(1_000_000_000).unwrap();

        // Should not panic for existing account
        svm.assert_account_exists(&account.pubkey());
    }

    #[test]
    #[should_panic(expected = "Expected account")]
    fn test_assert_account_exists_fails() {
        let svm = LiteSVM::new();
        let nonexistent = Pubkey::new_unique();

        // Should panic for non-existent account
        svm.assert_account_exists(&nonexistent);
    }

    #[test]
    fn test_assert_token_balance() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&authority, 9).unwrap();
        let token_account = svm
            .create_associated_token_account(&mint.pubkey(), &authority)
            .unwrap();

        // Mint tokens
        let amount = 1_000_000;
        svm.mint_to(&mint.pubkey(), &token_account, &authority, amount)
            .unwrap();

        // Should not panic with correct balance
        svm.assert_token_balance(&token_account, amount);
    }

    #[test]
    #[should_panic(expected = "Token balance mismatch")]
    fn test_assert_token_balance_fails() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&authority, 9).unwrap();
        let token_account = svm
            .create_associated_token_account(&mint.pubkey(), &authority)
            .unwrap();

        // Mint 1000 tokens
        svm.mint_to(&mint.pubkey(), &token_account, &authority, 1000)
            .unwrap();

        // Should panic when expecting wrong balance
        svm.assert_token_balance(&token_account, 2000);
    }

    #[test]
    fn test_assert_sol_balance() {
        let mut svm = LiteSVM::new();
        let lamports = 5_000_000_000;
        let account = svm.create_funded_account(lamports).unwrap();

        // Should not panic with correct balance
        svm.assert_sol_balance(&account.pubkey(), lamports);
    }

    #[test]
    #[should_panic(expected = "SOL balance mismatch")]
    fn test_assert_sol_balance_fails() {
        let mut svm = LiteSVM::new();
        let account = svm.create_funded_account(1_000_000_000).unwrap();

        // Should panic when expecting wrong balance
        svm.assert_sol_balance(&account.pubkey(), 2_000_000_000);
    }

    #[test]
    fn test_assert_sol_balance_zero_for_nonexistent() {
        let svm = LiteSVM::new();
        let nonexistent = Pubkey::new_unique();

        // Should not panic when expecting 0 for non-existent account
        svm.assert_sol_balance(&nonexistent, 0);
    }

    #[test]
    fn test_assert_mint_supply() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&authority, 9).unwrap();
        let token_account = svm
            .create_associated_token_account(&mint.pubkey(), &authority)
            .unwrap();

        // Mint tokens
        let amount = 5_000_000;
        svm.mint_to(&mint.pubkey(), &token_account, &authority, amount)
            .unwrap();

        // Should not panic with correct supply
        svm.assert_mint_supply(&mint.pubkey(), amount);
    }

    #[test]
    #[should_panic(expected = "Mint supply mismatch")]
    fn test_assert_mint_supply_fails() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&authority, 9).unwrap();
        let token_account = svm
            .create_associated_token_account(&mint.pubkey(), &authority)
            .unwrap();

        // Mint 100 tokens
        svm.mint_to(&mint.pubkey(), &token_account, &authority, 100)
            .unwrap();

        // Should panic when expecting wrong supply
        svm.assert_mint_supply(&mint.pubkey(), 200);
    }

    #[test]
    fn test_assert_mint_supply_zero_for_new_mint() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&authority, 9).unwrap();

        // New mint should have 0 supply
        svm.assert_mint_supply(&mint.pubkey(), 0);
    }

    #[test]
    fn test_assert_account_owner() {
        let mut svm = LiteSVM::new();
        let owner = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&owner, 9).unwrap();

        // Mint should be owned by token program
        svm.assert_account_owner(&mint.pubkey(), &spl_token::id());
    }

    #[test]
    #[should_panic(expected = "Account owner mismatch")]
    fn test_assert_account_owner_fails() {
        let mut svm = LiteSVM::new();
        let owner = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&owner, 9).unwrap();

        // Should panic when expecting wrong owner
        let wrong_owner = Pubkey::new_unique();
        svm.assert_account_owner(&mint.pubkey(), &wrong_owner);
    }

    #[test]
    fn test_assert_account_data_len() {
        let mut svm = LiteSVM::new();
        let owner = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&owner, 9).unwrap();

        // Mint account data is 82 bytes
        svm.assert_account_data_len(&mint.pubkey(), 82);
    }

    #[test]
    #[should_panic(expected = "Account data length mismatch")]
    fn test_assert_account_data_len_fails() {
        let mut svm = LiteSVM::new();
        let owner = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&owner, 9).unwrap();

        // Should panic when expecting wrong length
        svm.assert_account_data_len(&mint.pubkey(), 100);
    }

    #[test]
    fn test_assert_account_data_len_token_account() {
        let mut svm = LiteSVM::new();
        let owner = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&owner, 9).unwrap();
        let token_account = svm.create_token_account(&mint.pubkey(), &owner).unwrap();

        // Token account data is 165 bytes
        svm.assert_account_data_len(&token_account.pubkey(), 165);
    }
}