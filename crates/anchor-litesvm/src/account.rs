use anchor_lang::AccountDeserialize;
use litesvm::LiteSVM;
use solana_program::pubkey::Pubkey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountError {
    #[error("Account not found at address: {0}")]
    AccountNotFound(Pubkey),

    #[error("Failed to deserialize account: {0}")]
    DeserializationError(String),

    #[error("Account discriminator mismatch")]
    DiscriminatorMismatch,
}

/// Fetches and deserializes an Anchor account from LiteSVM
///
/// This function:
/// 1. Retrieves the account data from LiteSVM
/// 2. Deserializes it using Anchor's AccountDeserialize trait
/// 3. Handles the 8-byte discriminator that Anchor prepends to account data
pub fn get_anchor_account<T>(
    svm: &LiteSVM,
    address: &Pubkey,
) -> Result<T, AccountError>
where
    T: AccountDeserialize,
{
    // Get the account from LiteSVM
    let account = svm
        .get_account(address)
        .ok_or(AccountError::AccountNotFound(*address))?;

    // Deserialize using Anchor's method
    // Note: Anchor accounts have an 8-byte discriminator at the beginning
    let mut data_slice: &[u8] = &account.data;
    T::try_deserialize(&mut data_slice)
        .map_err(|e| AccountError::DeserializationError(e.to_string()))
}

/// Fetches and deserializes an Anchor account without discriminator check
///
/// Use this for accounts that don't have the standard Anchor discriminator
/// (e.g., some PDAs or custom account layouts)
///
/// Note: `try_deserialize_unchecked` already handles skipping the discriminator
/// internally, so we pass the full account data to it.
pub fn get_anchor_account_unchecked<T>(
    svm: &LiteSVM,
    address: &Pubkey,
) -> Result<T, AccountError>
where
    T: AccountDeserialize,
{
    // Get the account from LiteSVM
    let account = svm
        .get_account(address)
        .ok_or(AccountError::AccountNotFound(*address))?;

    // Deserialize without discriminator check
    // Note: try_deserialize_unchecked handles the discriminator internally
    let mut data_slice: &[u8] = &account.data;
    T::try_deserialize_unchecked(&mut data_slice)
        .map_err(|e| AccountError::DeserializationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::Discriminator;
    use borsh::{BorshDeserialize, BorshSerialize};

    #[test]
    fn test_account_error_display() {
        let addr = Pubkey::new_unique();
        let err = AccountError::AccountNotFound(addr);
        assert!(err.to_string().contains(&addr.to_string()));
    }

    // Test account struct that mimics Anchor's account structure
    #[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
    struct TestAccount {
        pub value: u64,
        pub owner: Pubkey,
    }

    impl Discriminator for TestAccount {
        const DISCRIMINATOR: &'static [u8] = &[1, 2, 3, 4, 5, 6, 7, 8];
    }

    impl anchor_lang::AccountDeserialize for TestAccount {
        fn try_deserialize(buf: &mut &[u8]) -> Result<Self, anchor_lang::error::Error> {
            // Check discriminator
            if buf.len() < 8 {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into());
            }

            let disc = &buf[0..8];
            if disc != Self::DISCRIMINATOR {
                return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into());
            }

            // Skip discriminator and deserialize
            *buf = &buf[8..];
            BorshDeserialize::deserialize(buf)
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
        }

        fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, anchor_lang::error::Error> {
            // Skip discriminator without checking
            if buf.len() < 8 {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into());
            }
            *buf = &buf[8..];
            BorshDeserialize::deserialize(buf)
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
        }
    }

    #[test]
    fn test_get_anchor_account_with_discriminator() {
        let mut svm = LiteSVM::new();
        let addr = Pubkey::new_unique();

        // Create test account data with discriminator
        let test_account = TestAccount {
            value: 42,
            owner: Pubkey::new_unique(),
        };

        let mut data = Vec::new();
        // Add discriminator
        data.extend_from_slice(TestAccount::DISCRIMINATOR);
        // Add serialized account data
        BorshSerialize::serialize(&test_account, &mut data).unwrap();

        // Create account in LiteSVM
        svm.set_account(
            addr,
            solana_sdk::account::Account {
                lamports: 1_000_000,
                data,
                owner: Pubkey::new_unique(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // Test get_anchor_account (with discriminator check)
        let retrieved: TestAccount = get_anchor_account(&svm, &addr).unwrap();
        assert_eq!(retrieved.value, 42);
        assert_eq!(retrieved.owner, test_account.owner);
    }

    #[test]
    fn test_get_anchor_account_unchecked() {
        let mut svm = LiteSVM::new();
        let addr = Pubkey::new_unique();

        // Create test account data with discriminator
        let test_account = TestAccount {
            value: 100,
            owner: Pubkey::new_unique(),
        };

        let mut data = Vec::new();
        // Add discriminator (even though we'll skip it)
        data.extend_from_slice(TestAccount::DISCRIMINATOR);
        // Add serialized account data
        BorshSerialize::serialize(&test_account, &mut data).unwrap();

        // Create account in LiteSVM
        svm.set_account(
            addr,
            solana_sdk::account::Account {
                lamports: 1_000_000,
                data,
                owner: Pubkey::new_unique(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // Test get_anchor_account_unchecked (without discriminator check)
        let retrieved: TestAccount = get_anchor_account_unchecked(&svm, &addr).unwrap();
        assert_eq!(retrieved.value, 100);
        assert_eq!(retrieved.owner, test_account.owner);
    }

    #[test]
    fn test_get_anchor_account_discriminator_mismatch() {
        let mut svm = LiteSVM::new();
        let addr = Pubkey::new_unique();

        // Create test account data with WRONG discriminator
        let test_account = TestAccount {
            value: 42,
            owner: Pubkey::new_unique(),
        };

        let mut data = Vec::new();
        // Add WRONG discriminator
        data.extend_from_slice(&[9, 9, 9, 9, 9, 9, 9, 9]);
        // Add serialized account data
        BorshSerialize::serialize(&test_account, &mut data).unwrap();

        // Create account in LiteSVM
        svm.set_account(
            addr,
            solana_sdk::account::Account {
                lamports: 1_000_000,
                data,
                owner: Pubkey::new_unique(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // Test get_anchor_account should FAIL with wrong discriminator
        let result: Result<TestAccount, AccountError> = get_anchor_account(&svm, &addr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AccountError::DeserializationError(_)));
    }

    #[test]
    fn test_get_anchor_account_not_found() {
        let svm = LiteSVM::new();
        let addr = Pubkey::new_unique();

        // Try to get non-existent account
        let result: Result<TestAccount, AccountError> = get_anchor_account(&svm, &addr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AccountError::AccountNotFound(_)));
    }

    #[test]
    fn test_get_anchor_account_unchecked_still_works_with_wrong_discriminator() {
        let mut svm = LiteSVM::new();
        let addr = Pubkey::new_unique();

        // Create test account data with WRONG discriminator
        let test_account = TestAccount {
            value: 99,
            owner: Pubkey::new_unique(),
        };

        let mut data = Vec::new();
        // Add WRONG discriminator (but unchecked should skip it anyway)
        data.extend_from_slice(&[9, 9, 9, 9, 9, 9, 9, 9]);
        // Add serialized account data
        BorshSerialize::serialize(&test_account, &mut data).unwrap();

        // Create account in LiteSVM
        svm.set_account(
            addr,
            solana_sdk::account::Account {
                lamports: 1_000_000,
                data,
                owner: Pubkey::new_unique(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // Test get_anchor_account_unchecked should SUCCEED even with wrong discriminator
        let retrieved: TestAccount = get_anchor_account_unchecked(&svm, &addr).unwrap();
        assert_eq!(retrieved.value, 99);
        assert_eq!(retrieved.owner, test_account.owner);
    }
}