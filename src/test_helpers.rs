//! Test helper utilities for common account operations
//!
//! This module provides convenient methods for creating and managing test accounts,
//! token mints, and associated token accounts.

use crate::AnchorContext;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use std::error::Error;

/// Test helper methods for AnchorContext
pub trait TestHelpers {
    /// Create a new funded keypair
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, TestHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let account = ctx.create_funded_account(1_000_000_000).unwrap();
    /// ```
    fn create_funded_account(&mut self, lamports: u64) -> Result<Keypair, Box<dyn Error>>;

    /// Create multiple funded keypairs
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, TestHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let accounts = ctx.create_funded_accounts(3, 1_000_000_000).unwrap();
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
    /// # use anchor_litesvm::{AnchorContext, TestHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    /// let mint = ctx.create_token_mint(&authority, 9).unwrap();
    /// ```
    fn create_token_mint(
        &mut self,
        authority: &Keypair,
        decimals: u8,
    ) -> Result<Keypair, Box<dyn Error>>;

    /// Create an associated token account and optionally mint tokens to it
    ///
    /// # Example
    /// ```ignore
    /// # use anchor_litesvm::{AnchorContext, TestHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    /// # let mint = ctx.create_token_mint(&authority, 9).unwrap();
    /// let owner = ctx.create_funded_account(1_000_000_000).unwrap();
    /// let ata = ctx.create_token_account(
    ///     &owner,
    ///     &mint.pubkey(),
    ///     Some((1_000_000_000, &authority)) // Mint 1 token with 9 decimals
    /// ).unwrap();
    /// ```
    fn create_token_account(
        &mut self,
        owner: &Keypair,
        mint: &Pubkey,
        mint_amount: Option<(u64, &Keypair)>, // (amount, mint_authority)
    ) -> Result<Pubkey, Box<dyn Error>>;

    /// Airdrop SOL to multiple accounts
    ///
    /// # Example
    /// ```ignore
    /// # use anchor_litesvm::{AnchorContext, TestHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let account1 = Keypair::new();
    /// let account2 = Keypair::new();
    /// ctx.batch_airdrop(&[&account1.pubkey(), &account2.pubkey()], 1_000_000_000).unwrap();
    /// ```
    fn batch_airdrop(&mut self, pubkeys: &[&Pubkey], lamports: u64)
        -> Result<(), Box<dyn Error>>;
}

impl TestHelpers for AnchorContext {
    fn create_funded_account(&mut self, lamports: u64) -> Result<Keypair, Box<dyn Error>> {
        let keypair = Keypair::new();
        self.svm.airdrop(&keypair.pubkey(), lamports).map_err(|e| format!("Airdrop failed: {:?}", e))?;
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
        use litesvm_token::spl_token;

        let mint = Keypair::new();
        let rent = self.svm.minimum_balance_for_rent_exemption(82); // Mint::LEN

        let instructions = vec![
            solana_sdk::system_instruction::create_account(
                &authority.pubkey(),
                &mint.pubkey(),
                rent,
                82,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                &authority.pubkey(),
                None,
                decimals,
            )?,
        ];

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&authority.pubkey()),
            &[authority, &mint],
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).map_err(|e| format!("Transaction failed: {:?}", e))?;
        Ok(mint)
    }

    fn create_token_account(
        &mut self,
        owner: &Keypair,
        mint: &Pubkey,
        mint_amount: Option<(u64, &Keypair)>,
    ) -> Result<Pubkey, Box<dyn Error>> {
        use litesvm_token::spl_token;

        let ata = get_associated_token_address(&owner.pubkey(), mint);

        // Create ATA
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &owner.pubkey(),
                &owner.pubkey(),
                mint,
                &spl_token::id(),
            );

        let tx = Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&owner.pubkey()),
            &[owner],
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx).map_err(|e| format!("Transaction failed: {:?}", e))?;

        // Mint tokens if requested
        if let Some((amount, mint_authority)) = mint_amount {
            let mint_to_ix = spl_token::instruction::mint_to(
                &spl_token::id(),
                mint,
                &ata,
                &mint_authority.pubkey(),
                &[],
                amount,
            )?;

            let tx = Transaction::new_signed_with_payer(
                &[mint_to_ix],
                Some(&mint_authority.pubkey()),
                &[mint_authority],
                self.svm.latest_blockhash(),
            );
            self.svm.send_transaction(tx).map_err(|e| format!("Transaction failed: {:?}", e))?;
        }

        Ok(ata)
    }

    fn batch_airdrop(
        &mut self,
        pubkeys: &[&Pubkey],
        lamports: u64,
    ) -> Result<(), Box<dyn Error>> {
        for pubkey in pubkeys {
            self.svm.airdrop(pubkey, lamports).map_err(|e| format!("Airdrop failed: {:?}", e))?;
        }
        Ok(())
    }
}

/// Additional helper functions for token operations
pub mod token {
    use super::*;
    use litesvm_token::spl_token;
    use solana_program_pack::Pack;

    /// Get the balance of a token account
    pub fn get_token_balance(
        ctx: &AnchorContext,
        token_account: &Pubkey,
    ) -> Result<u64, Box<dyn Error>> {
        let account = ctx
            .svm
            .get_account(token_account)
            .ok_or("Token account not found")?;
        let token_account = spl_token::state::Account::unpack(&account.data)?;
        Ok(token_account.amount)
    }

    /// Check if a token account exists and return its balance
    pub fn get_token_balance_safe(ctx: &AnchorContext, token_account: &Pubkey) -> u64 {
        ctx.svm
            .get_account(token_account)
            .and_then(|account| spl_token::state::Account::unpack(&account.data).ok())
            .map(|token_account| token_account.amount)
            .unwrap_or(0)
    }
}