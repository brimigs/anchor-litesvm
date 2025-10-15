//! Test helper utilities for common account operations
//!
//! This module provides convenient methods for creating and managing test accounts,
//! token mints, and associated token accounts.

use litesvm::LiteSVM;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use std::error::Error;

/// Test helper methods for LiteSVM
pub trait TestHelpers {
    /// Create a new funded keypair
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # let mut svm = LiteSVM::new();
    /// let account = svm.create_funded_account(1_000_000_000).unwrap();
    /// ```
    fn create_funded_account(&mut self, lamports: u64) -> Result<Keypair, Box<dyn Error>>;

    /// Create multiple funded keypairs
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # let mut svm = LiteSVM::new();
    /// let accounts = svm.create_funded_accounts(3, 1_000_000_000).unwrap();
    /// assert_eq!(accounts.len(), 3);
    /// ```
    fn create_funded_accounts(
        &mut self,
        count: usize,
        lamports: u64,
    ) -> Result<Vec<Keypair>, Box<dyn Error>>;

    /// Create and initialize a token mint
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let authority = Keypair::new();
    /// let mint = svm.create_token_mint(&authority, 9).unwrap();
    /// ```
    fn create_token_mint(
        &mut self,
        authority: &Keypair,
        decimals: u8,
    ) -> Result<Keypair, Box<dyn Error>>;

    /// Create a token account for a mint
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_sdk::signature::{Keypair, Signer};
    /// # let mut svm = LiteSVM::new();
    /// # let owner = Keypair::new();
    /// # let mint = Keypair::new();
    /// let token_account = svm.create_token_account(&mint.pubkey(), &owner).unwrap();
    /// ```
    fn create_token_account(
        &mut self,
        mint: &Pubkey,
        owner: &Keypair,
    ) -> Result<Keypair, Box<dyn Error>>;

    /// Create an associated token account
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_sdk::signature::{Keypair, Signer};
    /// # let mut svm = LiteSVM::new();
    /// # let owner = Keypair::new();
    /// # let mint = Keypair::new();
    /// let ata = svm.create_associated_token_account(&mint.pubkey(), &owner).unwrap();
    /// ```
    fn create_associated_token_account(
        &mut self,
        mint: &Pubkey,
        owner: &Keypair,
    ) -> Result<Pubkey, Box<dyn Error>>;

    /// Mint tokens to an account
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_sdk::signature::{Keypair, Signer};
    /// # use solana_program::pubkey::Pubkey;
    /// # let mut svm = LiteSVM::new();
    /// # let mint = Keypair::new();
    /// # let token_account = Pubkey::new_unique();
    /// # let authority = Keypair::new();
    /// svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000_000).unwrap();
    /// ```
    fn mint_to(
        &mut self,
        mint: &Pubkey,
        account: &Pubkey,
        authority: &Keypair,
        amount: u64,
    ) -> Result<(), Box<dyn Error>>;

    /// Derive a program-derived address
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// let (pda, bump) = svm.derive_pda(&[b"seed"], &program_id);
    /// ```
    fn derive_pda(&self, seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8);

    /// Get a program-derived address (convenience wrapper for Pubkey::find_program_address)
    ///
    /// This is a more convenient version that returns just the PDA without the bump.
    /// Use this when you don't need the bump seed.
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::{Keypair, Signer};
    /// # let svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// # let maker = Keypair::new();
    /// # let seed = 42u64;
    /// // Simple usage with multiple seeds
    /// let escrow_pda = svm.get_pda(
    ///     &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
    ///     &program_id
    /// );
    /// ```
    fn get_pda(&self, seeds: &[&[u8]], program_id: &Pubkey) -> Pubkey {
        let (pda, _bump) = self.derive_pda(seeds, program_id);
        pda
    }

    /// Get a program-derived address with bump (alias for derive_pda for consistency)
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TestHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// let (pda, bump) = svm.get_pda_with_bump(&[b"seed"], &program_id);
    /// ```
    fn get_pda_with_bump(&self, seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
        self.derive_pda(seeds, program_id)
    }

    /// Get the current slot
    fn get_current_slot(&self) -> u64;

    /// Advance the slot by a specified amount
    fn advance_slot(&mut self, slots: u64);
}

impl TestHelpers for LiteSVM {
    fn create_funded_account(&mut self, lamports: u64) -> Result<Keypair, Box<dyn Error>> {
        let keypair = Keypair::new();
        self.airdrop(&keypair.pubkey(), lamports)
            .map_err(|e| format!("Failed to airdrop: {:?}", e))?;
        Ok(keypair)
    }

    fn create_funded_accounts(
        &mut self,
        count: usize,
        lamports: u64,
    ) -> Result<Vec<Keypair>, Box<dyn Error>> {
        let mut accounts = Vec::with_capacity(count);
        for _ in 0..count {
            accounts.push(self.create_funded_account(lamports)?);
        }
        Ok(accounts)
    }

    fn create_token_mint(
        &mut self,
        authority: &Keypair,
        decimals: u8,
    ) -> Result<Keypair, Box<dyn Error>> {
        let mint = Keypair::new();

        // Calculate rent for mint account
        let rent = self.minimum_balance_for_rent_exemption(82);

        // Create mint account
        let create_account_ix = solana_program::system_instruction::create_account(
            &authority.pubkey(),
            &mint.pubkey(),
            rent,
            82,
            &spl_token::id(),
        );

        // Initialize mint
        let init_mint_ix = spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint.pubkey(),
            &authority.pubkey(),
            None,
            decimals,
        )?;

        // Send transaction
        let tx = Transaction::new_signed_with_payer(
            &[create_account_ix, init_mint_ix],
            Some(&authority.pubkey()),
            &[authority, &mint],
            self.latest_blockhash(),
        );

        self.send_transaction(tx)
            .map_err(|e| format!("Failed to create mint: {:?}", e.err))?;
        Ok(mint)
    }

    fn create_token_account(
        &mut self,
        mint: &Pubkey,
        owner: &Keypair,
    ) -> Result<Keypair, Box<dyn Error>> {
        let token_account = Keypair::new();

        // Calculate rent for token account
        let rent = self.minimum_balance_for_rent_exemption(165);

        // Create account
        let create_account_ix = solana_program::system_instruction::create_account(
            &owner.pubkey(),
            &token_account.pubkey(),
            rent,
            165,
            &spl_token::id(),
        );

        // Initialize token account
        let init_account_ix = spl_token::instruction::initialize_account(
            &spl_token::id(),
            &token_account.pubkey(),
            mint,
            &owner.pubkey(),
        )?;

        // Send transaction
        let tx = Transaction::new_signed_with_payer(
            &[create_account_ix, init_account_ix],
            Some(&owner.pubkey()),
            &[owner, &token_account],
            self.latest_blockhash(),
        );

        self.send_transaction(tx)
            .map_err(|e| format!("Failed to create token account: {:?}", e.err))?;
        Ok(token_account)
    }

    fn create_associated_token_account(
        &mut self,
        mint: &Pubkey,
        owner: &Keypair,
    ) -> Result<Pubkey, Box<dyn Error>> {
        let ata = get_associated_token_address(&owner.pubkey(), mint);

        // Create ATA instruction
        let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &owner.pubkey(),
            &owner.pubkey(),
            mint,
            &spl_token::id(),
        );

        // Send transaction
        let tx = Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&owner.pubkey()),
            &[owner],
            self.latest_blockhash(),
        );

        self.send_transaction(tx)
            .map_err(|e| format!("Failed to create ATA: {:?}", e.err))?;
        Ok(ata)
    }

    fn mint_to(
        &mut self,
        mint: &Pubkey,
        account: &Pubkey,
        authority: &Keypair,
        amount: u64,
    ) -> Result<(), Box<dyn Error>> {
        // Create mint_to instruction
        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            account,
            &authority.pubkey(),
            &[],
            amount,
        )?;

        // Send transaction
        let tx = Transaction::new_signed_with_payer(
            &[mint_to_ix],
            Some(&authority.pubkey()),
            &[authority],
            self.latest_blockhash(),
        );

        self.send_transaction(tx)
            .map_err(|e| format!("Failed to mint tokens: {:?}", e.err))?;
        Ok(())
    }

    fn derive_pda(&self, seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(seeds, program_id)
    }

    fn get_current_slot(&self) -> u64 {
        // LiteSVM doesn't have get_clock, use slot directly
        self.get_sysvar::<solana_program::clock::Clock>().slot
    }

    fn advance_slot(&mut self, slots: u64) {
        let current_slot = self.get_sysvar::<solana_program::clock::Clock>().slot;
        for i in 0..slots {
            self.warp_to_slot(current_slot + i + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_pack::Pack;
    use solana_sdk::signature::Signer;

    #[test]
    fn test_create_funded_account() {
        let mut svm = LiteSVM::new();
        let lamports = 1_000_000_000;

        let account = svm.create_funded_account(lamports).unwrap();

        // Verify account exists and has correct balance
        let balance = svm.get_balance(&account.pubkey()).unwrap();
        assert_eq!(balance, lamports);
    }

    #[test]
    fn test_create_funded_accounts() {
        let mut svm = LiteSVM::new();
        let count = 5;
        let lamports = 500_000_000;

        let accounts = svm.create_funded_accounts(count, lamports).unwrap();

        // Verify correct number of accounts created
        assert_eq!(accounts.len(), count);

        // Verify each account has correct balance
        for account in &accounts {
            let balance = svm.get_balance(&account.pubkey()).unwrap();
            assert_eq!(balance, lamports);
        }

        // Verify all accounts have unique pubkeys
        let mut pubkeys: Vec<_> = accounts.iter().map(|k| k.pubkey()).collect();
        pubkeys.sort();
        pubkeys.dedup();
        assert_eq!(pubkeys.len(), count);
    }

    #[test]
    fn test_create_token_mint() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let decimals = 9;

        let mint = svm.create_token_mint(&authority, decimals).unwrap();

        // Verify mint account exists
        let mint_account = svm.get_account(&mint.pubkey()).unwrap();
        assert_eq!(mint_account.owner, spl_token::id());

        // Verify mint data is correct
        let mint_data = spl_token::state::Mint::unpack(&mint_account.data).unwrap();
        assert_eq!(mint_data.decimals, decimals);
        assert_eq!(mint_data.mint_authority, Some(authority.pubkey()).into());
        assert_eq!(mint_data.supply, 0);
    }

    #[test]
    fn test_create_token_account() {
        let mut svm = LiteSVM::new();
        let owner = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&owner, 9).unwrap();

        let token_account = svm.create_token_account(&mint.pubkey(), &owner).unwrap();

        // Verify token account exists
        let account = svm.get_account(&token_account.pubkey()).unwrap();
        assert_eq!(account.owner, spl_token::id());

        // Verify token account data
        let token_data = spl_token::state::Account::unpack(&account.data).unwrap();
        assert_eq!(token_data.mint, mint.pubkey());
        assert_eq!(token_data.owner, owner.pubkey());
        assert_eq!(token_data.amount, 0);
    }

    #[test]
    fn test_create_associated_token_account() {
        let mut svm = LiteSVM::new();
        let owner = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&owner, 9).unwrap();

        let ata = svm
            .create_associated_token_account(&mint.pubkey(), &owner)
            .unwrap();

        // Verify ATA is at expected address
        let expected_ata = get_associated_token_address(&owner.pubkey(), &mint.pubkey());
        assert_eq!(ata, expected_ata);

        // Verify ATA account exists
        let account = svm.get_account(&ata).unwrap();
        assert_eq!(account.owner, spl_token::id());

        // Verify ATA data
        let token_data = spl_token::state::Account::unpack(&account.data).unwrap();
        assert_eq!(token_data.mint, mint.pubkey());
        assert_eq!(token_data.owner, owner.pubkey());
        assert_eq!(token_data.amount, 0);
    }

    #[test]
    fn test_mint_to() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&authority, 9).unwrap();
        let token_account = svm
            .create_associated_token_account(&mint.pubkey(), &authority)
            .unwrap();

        let amount = 1_000_000_000;
        svm.mint_to(&mint.pubkey(), &token_account, &authority, amount)
            .unwrap();

        // Verify token account balance
        let account = svm.get_account(&token_account).unwrap();
        let token_data = spl_token::state::Account::unpack(&account.data).unwrap();
        assert_eq!(token_data.amount, amount);

        // Verify mint supply increased
        let mint_account = svm.get_account(&mint.pubkey()).unwrap();
        let mint_data = spl_token::state::Mint::unpack(&mint_account.data).unwrap();
        assert_eq!(mint_data.supply, amount);
    }

    #[test]
    fn test_mint_to_multiple_times() {
        let mut svm = LiteSVM::new();
        let authority = svm.create_funded_account(10_000_000_000).unwrap();
        let mint = svm.create_token_mint(&authority, 9).unwrap();
        let token_account = svm
            .create_associated_token_account(&mint.pubkey(), &authority)
            .unwrap();

        // Mint tokens multiple times
        svm.mint_to(&mint.pubkey(), &token_account, &authority, 100_000)
            .unwrap();
        svm.mint_to(&mint.pubkey(), &token_account, &authority, 200_000)
            .unwrap();
        svm.mint_to(&mint.pubkey(), &token_account, &authority, 300_000)
            .unwrap();

        // Verify cumulative balance
        let account = svm.get_account(&token_account).unwrap();
        let token_data = spl_token::state::Account::unpack(&account.data).unwrap();
        assert_eq!(token_data.amount, 600_000);
    }

    #[test]
    fn test_derive_pda() {
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let seeds: &[&[u8]] = &[b"test", b"seeds"];

        let (pda, bump) = svm.derive_pda(seeds, &program_id);

        // Verify PDA is valid (off-curve)
        assert!(!pda.is_on_curve());

        // Verify we can recreate the PDA with the bump
        let expected_pda =
            Pubkey::create_program_address(&[seeds[0], seeds[1], &[bump]], &program_id).unwrap();
        assert_eq!(pda, expected_pda);
    }

    #[test]
    fn test_get_pda() {
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let seeds: &[&[u8]] = &[b"vault", b"test"];

        let pda = svm.get_pda(seeds, &program_id);

        // Verify PDA matches derive_pda result
        let (expected_pda, _) = svm.derive_pda(seeds, &program_id);
        assert_eq!(pda, expected_pda);
    }

    #[test]
    fn test_get_pda_with_bump() {
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let seeds: &[&[u8]] = &[b"escrow"];

        let (pda, bump) = svm.get_pda_with_bump(seeds, &program_id);

        // Verify matches derive_pda
        let (expected_pda, expected_bump) = svm.derive_pda(seeds, &program_id);
        assert_eq!(pda, expected_pda);
        assert_eq!(bump, expected_bump);
    }

    #[test]
    fn test_get_current_slot() {
        let svm = LiteSVM::new();

        let slot = svm.get_current_slot();

        // Initial slot should be 0
        assert_eq!(slot, 0);
    }

    #[test]
    fn test_advance_slot() {
        let mut svm = LiteSVM::new();

        let initial_slot = svm.get_current_slot();
        let slots_to_advance = 100;

        svm.advance_slot(slots_to_advance);

        let new_slot = svm.get_current_slot();
        assert_eq!(new_slot, initial_slot + slots_to_advance);
    }

    #[test]
    fn test_advance_slot_multiple_times() {
        let mut svm = LiteSVM::new();

        svm.advance_slot(10);
        assert_eq!(svm.get_current_slot(), 10);

        svm.advance_slot(25);
        assert_eq!(svm.get_current_slot(), 35);

        svm.advance_slot(5);
        assert_eq!(svm.get_current_slot(), 40);
    }
}