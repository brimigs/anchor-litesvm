//! Golden test: bundles_from_idl! output vs a hand-built Instruction.
// declare_program!'s expansion gates on-chain-only branches with
// `cfg(target_os = "solana")` and `cfg(feature = "idl-build")`; off-chain those
// compile out, but check-cfg doesn't know the names, so silence the noise here.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::Pubkey;
// `self` binds the crate name into this test's root scope; declare_program!'s
// generated modules reach `anchor_lang` via `super::`, so it must be nameable
// here (without it, the expansion fails with "no `anchor_lang` in the root").
use anchor_lang::{self, InstructionData, ToAccountMetas};

anchor_lang::declare_program!(vault);
anchor_litesvm::bundles_from_idl!(vault);

const VAULT_PROGRAM: Pubkey =
    Pubkey::from_str_const("6RviLVy2WPGm7QYfCuZq66vKWF58WVTNWfFE7RgWxcfP");

#[test]
fn deposit_bundle_matches_hand_built_instruction() {
    let user = Pubkey::new_unique();
    // Hand-derived, straight from the IDL seeds:
    let (vault_state, _) = Pubkey::find_program_address(&[b"state", user.as_ref()], &VAULT_PROGRAM);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[b"vault", vault_state.as_ref()], &VAULT_PROGRAM);

    // The bundle needs ONLY the root key; PDAs and system_program are
    // derived/injected.
    let bundle = DepositBundle { user };
    let accounts: vault::client::accounts::Deposit = bundle.into();
    assert_eq!(accounts.vault_state, vault_state);
    assert_eq!(accounts.vault, vault_pda);

    let metas_generated = accounts.to_account_metas(None);
    let hand = vault::client::accounts::Deposit {
        user,
        vault_state,
        vault: vault_pda,
        system_program: Pubkey::from_str_const("11111111111111111111111111111111"),
    };
    assert_eq!(metas_generated, hand.to_account_metas(None));

    // sha256("global:deposit")[..8], from the committed IDL's `deposit`
    // discriminator, followed by the u64-LE `amount` argument.
    let args = vault::client::args::Deposit { amount: 5 };
    let mut expected = vec![242, 35, 198, 137, 82, 225, 242, 182];
    expected.extend_from_slice(&5u64.to_le_bytes());
    assert_eq!(args.data(), expected);
}

#[test]
fn pda_helpers_match_find_program_address() {
    let user = Pubkey::new_unique();
    let (expect_state, expect_bump) =
        Pubkey::find_program_address(&[b"state", user.as_ref()], &VAULT_PROGRAM);
    assert_eq!(vault_state_pda(&user), (expect_state, expect_bump));
}

#[test]
fn injected_programs_lists_system_program() {
    assert!(injected_programs()
        .iter()
        .any(|(_, name)| *name == "system_program"));
}

#[test]
fn generated_bundles_ride_the_tx_builder() {
    use anchor_litesvm::{AnchorContext, Keypair, LiteSVM};

    // Build-only: the emitted no-op `Resolvable` is what lets a generated
    // bundle satisfy `Tx::build`'s bound; no program deploy needed to
    // prove the instruction assembles.
    let mut ctx = AnchorContext::new(LiteSVM::new(), vault::ID);
    let payer = Keypair::new();
    let user = Pubkey::new_unique();

    let signers = [&payer];
    let tx = ctx.tx(&signers).build(
        DepositBundle { user },
        vault::client::args::Deposit { amount: 7 },
    );
    drop(tx);
}

#[test]
fn build_ix_with_corrupts_exactly_one_derived_account() {
    use anchor_litesvm::{AnchorContext, LiteSVM};

    // The negative-path escape hatch: the bundle derives every account
    // honestly, then the closure swaps exactly the one under attack, with
    // the accounts struct's types intact.
    let mut ctx = AnchorContext::new(LiteSVM::new(), vault::ID);
    let user = Pubkey::new_unique();
    let wrong_vault_state = Pubkey::new_unique();

    let honest = ctx.program().build_ix(
        DepositBundle { user },
        vault::client::args::Deposit { amount: 7 },
    );
    let corrupted = ctx.program().build_ix_with(
        DepositBundle { user },
        vault::client::args::Deposit { amount: 7 },
        |accounts| accounts.vault_state = wrong_vault_state,
    );

    assert_eq!(honest.accounts.len(), corrupted.accounts.len());
    let diffs: Vec<usize> = honest
        .accounts
        .iter()
        .zip(&corrupted.accounts)
        .enumerate()
        .filter(|(_, (a, b))| a.pubkey != b.pubkey)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(diffs.len(), 1, "exactly one slot corrupted");
    assert_eq!(corrupted.accounts[diffs[0]].pubkey, wrong_vault_state);
    assert_eq!(honest.data, corrupted.data, "args untouched");
}
