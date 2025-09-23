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
        .ok_or_else(|| AccountError::AccountNotFound(*address))?;

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
        .ok_or_else(|| AccountError::AccountNotFound(*address))?;

    // Skip the discriminator (first 8 bytes) and deserialize
    if account.data.len() < 8 {
        return Err(AccountError::DeserializationError(
            "Account data too small for Anchor account".to_string(),
        ));
    }

    let mut data_slice: &[u8] = &account.data[8..];
    T::try_deserialize_unchecked(&mut data_slice)
        .map_err(|e| AccountError::DeserializationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_error_display() {
        let addr = Pubkey::new_unique();
        let err = AccountError::AccountNotFound(addr);
        assert!(err.to_string().contains(&addr.to_string()));
    }
}