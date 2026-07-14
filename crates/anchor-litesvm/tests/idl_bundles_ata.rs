//! Integration: an `associated_token::` account whose IDL `pda` carries a
//! `program` derives under THAT program (the ATA program), not the host program.
//! This is the end-to-end proof of the foreign-program PDA fix: a real
//! `declare_program!` + `bundles_from_idl!` pair, with the bundle's `From`
//! producing the canonical associated-token address.
// declare_program!'s expansion gates on-chain-only branches with
// `cfg(target_os = "solana")` and `cfg(feature = "idl-build")`; off-chain those
// compile out, but check-cfg doesn't know the names, so silence the noise here.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::Pubkey;
// `self` binds the crate name into this test's root scope; declare_program!'s
// generated modules reach `anchor_lang` via `super::`, so it must be nameable
// here (without it, the expansion fails with "no `anchor_lang` in the root").
use anchor_lang::{self, ToAccountMetas};

anchor_lang::declare_program!(ata_pool);
anchor_litesvm::bundles_from_idl!(ata_pool);

// The SPL Associated Token and Token program ids, the two 32-byte constants the
// IDL's `payer_ata` derivation depends on. `find_program_address([wallet,
// token_program, mint], ATA)` is the canonical associated-token address.
const ATA_PROGRAM: Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const TOKEN_PROGRAM: Pubkey = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const HOST_PROGRAM: Pubkey = Pubkey::from_str_const("6RviLVy2WPGm7QYfCuZq66vKWF58WVTNWfFE7RgWxcfP");

fn canonical_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM.as_ref(), mint.as_ref()],
        &ATA_PROGRAM,
    )
    .0
}

#[test]
fn payer_ata_derives_under_the_ata_program() {
    let payer = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // The bundle binds only the two caller keys; `payer_ata` is derived and
    // `system_program` injected.
    let bundle = CreateBundle { payer, mint };
    let accounts: ata_pool::client::accounts::Create = bundle.into();

    let expected = canonical_ata(&payer, &mint);
    assert_eq!(
        accounts.payer_ata, expected,
        "payer_ata must be the canonical associated-token address"
    );

    // The bug this closes: deriving the same seeds under the host program yields
    // a different address. The fix must NOT produce that.
    let under_host = Pubkey::find_program_address(
        &[payer.as_ref(), TOKEN_PROGRAM.as_ref(), mint.as_ref()],
        &HOST_PROGRAM,
    )
    .0;
    assert_ne!(
        accounts.payer_ata, under_host,
        "payer_ata must not derive under the host program"
    );

    // The generated account metas agree with a hand-built accounts struct.
    let hand = ata_pool::client::accounts::Create {
        payer,
        mint,
        payer_ata: expected,
        system_program: Pubkey::from_str_const("11111111111111111111111111111111"),
    };
    assert_eq!(accounts.to_account_metas(None), hand.to_account_metas(None));
}

#[test]
fn payer_ata_pda_helper_matches_the_canonical_address() {
    let payer = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    // The `<account>_pda` helper honors the same foreign program as `From`.
    assert_eq!(payer_ata_pda(&payer, &mint).0, canonical_ata(&payer, &mint));
}
