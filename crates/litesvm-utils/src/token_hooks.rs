//! Token-2022 Transfer Hook testing helpers.
//!
//! Testing a transfer-hook program means driving a *real* Token-2022
//! `transfer_checked`: the runtime sets the source account's `transferring`
//! flag and CPIs into the hook with the extra accounts resolved from the mint's
//! `ExtraAccountMetaList`. None of that can be fabricated by writing bytes (the
//! flag only exists mid-transfer), so unlike [`crate::tokens`] this module
//! drives the real Token-2022 / ATA instructions, parameterized for the
//! Token-2022 program.
//!
//! These cover the setup a hook test needs that classic-SPL
//! [`TestHelpers`](crate::TestHelpers) cannot express (those methods are
//! hard-wired to `spl-token`; [`token_balance`](crate::TestHelpers::token_balance)
//! even panics on a Token-2022 account):
//!
//!   * [`create_transfer_hook_mint`](TransferHookTesting::create_transfer_hook_mint):
//!     a Token-2022 mint carrying the `TransferHook` extension.
//!   * [`create_token2022_ata`](TransferHookTesting::create_token2022_ata) /
//!     [`mint_to_2022`](TransferHookTesting::mint_to_2022): holders under Token-2022.
//!   * [`token2022_balance`](TransferHookTesting::token2022_balance): a balance
//!     read that tolerates the extension trailer.
//!   * [`transfer_checked_with_hook_ix`](TransferHookTesting::transfer_checked_with_hook_ix):
//!     a `transfer_checked` whose extra accounts are resolved from the *current*
//!     on-chain `ExtraAccountMetaList`, so it works for any hook program without
//!     the test knowing that program's account schema.
//!
//! The seed prefix and resolution recipe live in the program's
//! `ExtraAccountMetaList`; this module reads it back rather than reimplementing
//! it, which is what keeps the transfer helper program-agnostic.

use crate::tokens::TOKEN_2022_ID;
use anchor_litesvm_compat::{Keypair, LiteSVM, Signer, Transaction};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use std::error::Error;

use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022_interface::{
    extension::{
        transfer_hook::instruction::initialize as initialize_transfer_hook, ExtensionType,
    },
    instruction::{initialize_mint2, mint_to, transfer_checked},
    state::{Account as Token2022Account, Mint as Token2022Mint},
};
use spl_transfer_hook_interface::offchain::add_extra_account_metas_for_execute;

/// Token-2022 transfer-hook testing helpers on [`LiteSVM`].
pub trait TransferHookTesting {
    /// Create and initialize a Token-2022 mint carrying the `TransferHook`
    /// extension pointed at `hook_program`. `authority` pays, owns the mint, and
    /// is the hook-update authority. One transaction: allocate the account sized
    /// for the extension, initialize the extension, initialize the mint.
    fn create_transfer_hook_mint(
        &mut self,
        authority: &Keypair,
        decimals: u8,
        hook_program: &Pubkey,
    ) -> Result<Keypair, Box<dyn Error>>;

    /// Like [`create_transfer_hook_mint`](Self::create_transfer_hook_mint), but
    /// at a caller-chosen mint keypair so the mint address (and everything
    /// derived from it) is deterministic. Pair with
    /// [`deterministic_keypair`](crate::naming::deterministic_keypair) to keep
    /// committed reports byte-reproducible.
    fn create_transfer_hook_mint_at(
        &mut self,
        authority: &Keypair,
        mint: &Keypair,
        decimals: u8,
        hook_program: &Pubkey,
    ) -> Result<(), Box<dyn Error>>;

    /// Create a Token-2022 associated token account for `(owner, mint)`,
    /// returning its address. The ATA program auto-sizes the account's
    /// `TransferHookAccount` extension, so the holder is transfer-ready.
    fn create_token2022_ata(
        &mut self,
        payer: &Keypair,
        owner: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Pubkey, Box<dyn Error>>;

    /// Mint `amount` of a Token-2022 `mint` to `account`.
    fn mint_to_2022(
        &mut self,
        mint: &Pubkey,
        account: &Pubkey,
        authority: &Keypair,
        amount: u64,
    ) -> Result<(), Box<dyn Error>>;

    /// Read a Token-2022 token account's amount, tolerating the extension
    /// trailer. `None` if the account doesn't exist or doesn't unpack.
    fn token2022_balance(&self, account: &Pubkey) -> Option<u64>;

    /// Build a `transfer_checked` that fires `hook_program`'s transfer hook,
    /// with the extra accounts resolved from the mint's current on-chain
    /// `ExtraAccountMetaList`. Program-agnostic: the resolution recipe is read
    /// from chain, so the caller does not need to know the hook's PDA schema.
    // Mirrors `spl_token_2022::instruction::transfer_checked`'s parameter list
    // (which carries the same allow) plus the hook program id.
    #[allow(clippy::too_many_arguments)]
    fn transfer_checked_with_hook_ix(
        &self,
        hook_program: &Pubkey,
        mint: &Pubkey,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, Box<dyn Error>>;
}

impl TransferHookTesting for LiteSVM {
    fn create_transfer_hook_mint(
        &mut self,
        authority: &Keypair,
        decimals: u8,
        hook_program: &Pubkey,
    ) -> Result<Keypair, Box<dyn Error>> {
        let mint = Keypair::new();
        self.create_transfer_hook_mint_at(authority, &mint, decimals, hook_program)?;
        Ok(mint)
    }

    fn create_transfer_hook_mint_at(
        &mut self,
        authority: &Keypair,
        mint: &Keypair,
        decimals: u8,
        hook_program: &Pubkey,
    ) -> Result<(), Box<dyn Error>> {
        let space = ExtensionType::try_calculate_account_len::<Token2022Mint>(&[
            ExtensionType::TransferHook,
        ])?;
        let rent = self.minimum_balance_for_rent_exemption(space);

        let create = solana_system_interface::instruction::create_account(
            &authority.pubkey(),
            &mint.pubkey(),
            rent,
            space as u64,
            &TOKEN_2022_ID,
        );
        let init_hook = initialize_transfer_hook(
            &TOKEN_2022_ID,
            &mint.pubkey(),
            Some(authority.pubkey()),
            Some(*hook_program),
        )?;
        let init_mint = initialize_mint2(
            &TOKEN_2022_ID,
            &mint.pubkey(),
            &authority.pubkey(),
            None,
            decimals,
        )?;

        let tx = Transaction::new_signed_with_payer(
            &[create, init_hook, init_mint],
            Some(&authority.pubkey()),
            &[authority, mint],
            self.latest_blockhash(),
        );
        self.send_transaction(tx)
            .map_err(|e| format!("Failed to create transfer-hook mint: {:?}", e.err))?;
        Ok(())
    }

    fn create_token2022_ata(
        &mut self,
        payer: &Keypair,
        owner: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Pubkey, Box<dyn Error>> {
        let ix = create_associated_token_account(&payer.pubkey(), owner, mint, &TOKEN_2022_ID);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[payer],
            self.latest_blockhash(),
        );
        self.send_transaction(tx)
            .map_err(|e| format!("Failed to create Token-2022 ATA: {:?}", e.err))?;
        Ok(get_associated_token_address_with_program_id(
            owner,
            mint,
            &TOKEN_2022_ID,
        ))
    }

    fn mint_to_2022(
        &mut self,
        mint: &Pubkey,
        account: &Pubkey,
        authority: &Keypair,
        amount: u64,
    ) -> Result<(), Box<dyn Error>> {
        let ix = mint_to(
            &TOKEN_2022_ID,
            mint,
            account,
            &authority.pubkey(),
            &[],
            amount,
        )?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[authority],
            self.latest_blockhash(),
        );
        self.send_transaction(tx)
            .map_err(|e| format!("Failed to mint Token-2022 tokens: {:?}", e.err))?;
        Ok(())
    }

    fn token2022_balance(&self, account: &Pubkey) -> Option<u64> {
        use spl_token_2022_interface::extension::StateWithExtensions;
        let acct = self.get_account(account)?;
        StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .ok()
            .map(|state| state.base.amount)
    }

    fn transfer_checked_with_hook_ix(
        &self,
        hook_program: &Pubkey,
        mint: &Pubkey,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, Box<dyn Error>> {
        let mut ix = transfer_checked(
            &TOKEN_2022_ID,
            source,
            mint,
            destination,
            authority,
            &[],
            amount,
            decimals,
        )?;

        // Resolve the hook's declared extra accounts from the *current* on-chain
        // ExtraAccountMetaList. The resolver is async only to stay client-agnostic;
        // our fetch is an in-memory SVM read, so the futures are always ready and
        // a poll-once executor drives it to completion.
        block_on(add_extra_account_metas_for_execute(
            &mut ix,
            hook_program,
            source,
            mint,
            destination,
            authority,
            amount,
            |address: Pubkey| core::future::ready(Ok(self.get_account(&address).map(|a| a.data))),
        ))
        .map_err(|e| format!("transfer-hook account resolution failed: {e}"))?;

        Ok(ix)
    }
}

/// Drive a future to completion synchronously. Sound here because the only
/// awaits inside the transfer-hook resolver are over already-ready SVM reads, so
/// the future never genuinely pends.
fn block_on<F: core::future::Future>(future: F) -> F::Output {
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    fn clone(_: *const ()) -> RawWaker {
        raw_waker()
    }
    fn noop(_: *const ()) {}
    fn raw_waker() -> RawWaker {
        RawWaker::new(
            core::ptr::null(),
            &RawWakerVTable::new(clone, noop, noop, noop),
        )
    }

    let waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}
