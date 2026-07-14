//! End-to-end guard for the deterministic starting-slot override.
//!
//! `LiteSVMBuilder::start_slot` / `AnchorLiteSVM::start_slot` let a test pin the
//! world's starting slot instead of riding litesvm's version-varying default
//! (0.14's `MAINNET_DEFAULT_SLOT`). The pin is applied before programs deploy,
//! because a program's visibility slot is fixed to the current slot at deploy
//! time: pinning after deploy would leave the program invisible
//! (`current_slot < effective_slot`). These tests prove a deployed program is
//! still invocable at the pinned slot, and that without an override the world
//! follows the engine.
#![allow(unexpected_cfgs)]
mod common;

// `self` binds the crate name so declare_program!'s generated modules can reach
// `anchor_lang` via `super::` (mirrors the book tests).
use anchor_lang::{self};
use anchor_litesvm::{AnchorLiteSVM, Signer};
use litesvm_utils::TestHelpers;

anchor_lang::declare_program!(vault);
anchor_litesvm::bundles_from_idl!(vault);

#[test]
fn program_is_visible_and_invocable_at_pinned_slot_zero() {
    let mut ctx = AnchorLiteSVM::new()
        .deploy_program(vault::ID, "vault", &common::fixture_bytes("vault"))
        .start_slot(0)
        .build();
    let alice = ctx.cast_actor("Alice");

    // The program deployed at the pinned slot executes (visibility holds); a
    // deploy-then-warp-back would fail here with the program unreachable.
    ctx.tx(&[&alice])
        .build(
            InitializeBundle {
                user: alice.pubkey(),
            },
            vault::client::args::Initialize {},
        )
        .send_ok();

    // The world is at exactly the slot we pinned, not the engine default.
    assert_eq!(ctx.svm.get_current_slot(), 0);
}

#[test]
fn engine_default_slot_is_followed_without_override() {
    let ctx =
        AnchorLiteSVM::build_with_program(vault::ID, "vault", &common::fixture_bytes("vault"));

    // No override: the world rides litesvm's default, a nonzero mainnet-era slot.
    assert_ne!(ctx.svm.get_current_slot(), 0);
}
