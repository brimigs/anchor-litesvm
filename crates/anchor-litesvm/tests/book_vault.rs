//! Book capture: deploy the vault program, run initialize + deposit as
//! Alice, and snapshot the rendered CPI tree (including the decoded
//! `Deposited` badge) into `book/src/captured/vault_deposit.txt`.
// declare_program!'s expansion gates on-chain-only branches with
// `cfg(target_os = "solana")` and `cfg(feature = "idl-build")`; off-chain those
// compile out, but check-cfg doesn't know the names, so silence the noise here.
#![allow(unexpected_cfgs)]
mod common;

// `self` binds the crate name into this test's root scope; declare_program!'s
// generated modules reach `anchor_lang` via `super::`, so it must be nameable
// here (without it, the expansion fails with "no `anchor_lang` in the root").
use anchor_lang::{self};
use anchor_litesvm::{AnchorLiteSVM, EventHelpers, Signer};

anchor_lang::declare_program!(vault);
anchor_litesvm::bundles_from_idl!(vault);

fn boot() -> anchor_litesvm::AnchorContext {
    let mut ctx =
        AnchorLiteSVM::build_with_program(vault::ID, "vault", &common::fixture_bytes("vault"));
    // Decode `Deposited` badges from the committed IDL.
    ctx.register_events_from_idl(include_str!("../idls/vault.json"));
    ctx
}

#[test]
fn vault_deposit_happy_path() {
    let mut ctx = boot();
    let alice = ctx.cast_actor("Alice");

    // initialize creates the vault_state + vault PDAs for Alice.
    ctx.tx(&[&alice])
        .build(
            InitializeBundle {
                user: alice.pubkey(),
            },
            vault::client::args::Initialize {},
        )
        .send_ok();

    // deposit 1 SOL; capture the rendered CPI tree (system transfer + Deposited badge).
    let result = ctx
        .tx(&[&alice])
        .build(
            DepositBundle {
                user: alice.pubkey(),
            },
            vault::client::args::Deposit {
                amount: 1_000_000_000,
            },
        )
        .send_ok();

    // The decoded event is assertable as a typed value.
    // declare_program! emits event structs under `vault::events`, matched
    // from the IDL's `types` entry with the same name as the `events`
    // discriminator entry (not `client::accounts`, which is instruction
    // account structs).
    let ev: vault::events::Deposited = result.parse_event().expect("Deposited event present");
    assert_eq!(ev.amount, 1_000_000_000);

    common::expect_capture("vault_deposit", &result.tree_string());
}

#[test]
fn vault_deposit_wrong_state_is_rejected() {
    let mut ctx = boot();
    let alice = ctx.cast_actor("Alice");
    let mallory = ctx.cast_actor("Mallory");

    // Mallory initializes her OWN vault: a real, program-owned, correctly
    // discriminated VaultState account exists at her PDA. This is the
    // confused-deputy setup: the substituted account is valid in every way
    // except that its seeds derive from Mallory, not Alice.
    ctx.tx(&[&mallory])
        .build(
            InitializeBundle {
                user: mallory.pubkey(),
            },
            vault::client::args::Initialize {},
        )
        .send_ok();

    // Alice initializes her own vault (the honest counterpart).
    ctx.tx(&[&alice])
        .build(
            InitializeBundle {
                user: alice.pubkey(),
            },
            vault::client::args::Initialize {},
        )
        .send_ok();

    // The bundle derives every account honestly; the closure then swaps
    // exactly the vault_state slot for Mallory's valid, initialized PDA.
    // Anchor loads it fine (right owner, right discriminator) and reaches
    // a seeds constraint derived from Alice's key, which rejects the
    // mismatch: ConstraintSeeds.
    let (mallory_state, _) = vault_state_pda(&mallory.pubkey());
    let honest = ctx.program().build_ix(
        DepositBundle {
            user: alice.pubkey(),
        },
        vault::client::args::Deposit {
            amount: 1_000_000_000,
        },
    );
    let ix = ctx.program().build_ix_with(
        DepositBundle {
            user: alice.pubkey(),
        },
        vault::client::args::Deposit {
            amount: 1_000_000_000,
        },
        |accounts| accounts.vault_state = mallory_state,
    );

    // Prove the mechanism: exactly one account slot differs from the
    // honest build, and it's the corrupted vault_state.
    let diffs: Vec<usize> = honest
        .accounts
        .iter()
        .zip(&ix.accounts)
        .enumerate()
        .filter(|(_, (a, b))| a.pubkey != b.pubkey)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(diffs.len(), 1, "exactly one slot corrupted");
    assert_eq!(ix.accounts[diffs[0]].pubkey, mallory_state);

    let result = ctx.send_err_named(ix, &[&alice], "ConstraintSeeds");
    common::expect_capture("vault_wrong_state", &result.tree_string());
}
