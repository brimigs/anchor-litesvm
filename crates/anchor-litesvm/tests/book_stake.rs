//! Typed client for the vendored `staking` program (`examples/staking/`).
//!
//! `staking` is anchor 0.31 (an mpl-core dependency pins it), and its IDL
//! embeds mpl-core's `Key` enum, which collides with `anchor_lang::Key` under
//! `declare_program!`'s glob imports. `make fixtures` runs the sanitize pass
//! (`anchor_litesvm::sanitize_idl`) over `idls/staking.json`, namespacing `Key`
//! to `StakingKey`, so the macros generate a typed client like any other
//! program. Every instruction here is a bundle plus typed args; the deep
//! mpl-core CPI tree is what the program does, not something we hand-build.
#![allow(unexpected_cfgs)]
mod common;

use anchor_lang::prelude::Pubkey;
use anchor_lang::{self};
use anchor_litesvm::{AnchorLiteSVM, Keypair, Signer};
use litesvm_utils::naming::deterministic_keypair;
use litesvm_utils::TestHelpers;

anchor_lang::declare_program!(staking);
anchor_litesvm::bundles_from_idl!(staking);

const MPL_CORE_ID: Pubkey = Pubkey::from_str_const("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
const TOKEN_ID: Pubkey = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Deploys both vendored programs and names the staking custom errors. The
/// framework has no errors-from-IDL helper yet, so `register_program_errors`
/// supplies the mapping (codes are declaration order from 6000, per
/// `error.rs`); that is what makes a failing leaf read as
/// `FreezePeriodNotElapsed` instead of `custom program error: 0x1775`.
fn boot() -> anchor_litesvm::AnchorContext {
    let mut ctx = AnchorLiteSVM::build_with_programs(&[
        (staking::ID, "staking", &common::fixture_bytes("staking")),
        (MPL_CORE_ID, "mpl_core", &common::fixture_bytes("mpl_core")),
    ]);
    ctx.register_program_errors(
        staking::ID,
        &[
            (6000, "InvalidOwner"),
            (6001, "InvalidUpdateAuthority"),
            (6002, "AlreadyStaked"),
            (6003, "AssetNotStaked"),
            (6004, "InvalidTimestamp"),
            (6005, "FreezePeriodNotElapsed"),
            (6006, "InvalidRewardsBps"),
            (6007, "NothingToClaim"),
        ],
    );
    ctx
}

/// Create a collection, initialize the staking config on it (500 bps rewards,
/// 7-day freeze), and mint an asset into it, leaving an asset ready to stake.
/// The asset keypairs are deterministic so the captured CPI trees (which print
/// pubkeys through the alias table) stay stable across runs.
fn setup(ctx: &mut anchor_litesvm::AnchorContext, admin: &Keypair) -> (Keypair, Keypair) {
    let collection = deterministic_keypair(&staking::ID.to_string(), "Collection");
    let asset = deterministic_keypair(&staking::ID.to_string(), "Asset");
    ctx.alias(collection.pubkey(), "Collection");
    ctx.alias(asset.pubkey(), "Asset");

    ctx.tx(&[admin, &collection])
        .build(
            CreateCollectionBundle {
                payer: admin.pubkey(),
                collection: collection.pubkey(),
            },
            staking::client::args::CreateCollection {
                name: "Stake Collection".into(),
                uri: "https://example.com/collection.json".into(),
            },
        )
        .send_ok();

    ctx.tx(&[admin])
        .build(
            InitializeBundle {
                admin: admin.pubkey(),
                collection: collection.pubkey(),
                token_program: TOKEN_ID,
            },
            staking::client::args::Initialize {
                rewards_bps: 500,
                freeze_period: 7,
            },
        )
        .send_ok();

    ctx.tx(&[admin, &asset])
        .build(
            MintAssetBundle {
                user: admin.pubkey(),
                asset: asset.pubkey(),
                collection: collection.pubkey(),
            },
            staking::client::args::MintAsset {
                name: "Stake Asset".into(),
                uri: "https://example.com/asset.json".into(),
            },
        )
        .send_ok();

    (collection, asset)
}

fn stake_bundle(admin: &Keypair, asset: &Keypair, collection: &Keypair) -> StakeBundle {
    StakeBundle {
        owner: admin.pubkey(),
        asset: asset.pubkey(),
        collection: collection.pubkey(),
    }
}

fn unstake_bundle(admin: &Keypair, asset: &Keypair, collection: &Keypair) -> UnstakeBundle {
    UnstakeBundle {
        owner: admin.pubkey(),
        asset: asset.pubkey(),
        collection: collection.pubkey(),
        token_program: TOKEN_ID,
    }
}

/// Stake an asset as the admin. `stake`'s CPI tree is the deepest in the book:
/// it invokes into `mpl_core` twice (an Attributes plugin add, then a
/// FreezeDelegate plugin add) to record `staked` / `staked_at` and freeze the
/// asset in place. The bundle derives `config` and `update_authority` from the
/// IDL's seeds; only the owner and the two mpl-core assets vary per call.
#[test]
fn stake_happy_path() {
    let mut ctx = boot();
    let admin = ctx.cast_actor("Alice");
    let (collection, asset) = setup(&mut ctx, &admin);

    let result = ctx
        .tx(&[&admin])
        .build(
            stake_bundle(&admin, &asset, &collection),
            staking::client::args::Stake {},
        )
        .send_ok();
    common::expect_capture("stake", &result.tree_string());
}

/// `unstake` reads the Clock, computes `staked_time = (now - staked_at) /
/// SECONDS_PER_DAY`, and requires `staked_time >= freeze_period` (7, set in
/// `setup`) before it will unfreeze the asset. One elapsed day is nowhere near
/// enough, so the instruction must bail on `FreezePeriodNotElapsed` (6005)
/// rather than touch the asset.
#[test]
fn unstake_before_freeze_is_rejected() {
    let mut ctx = boot();
    let admin = ctx.cast_actor("Alice");
    let (collection, asset) = setup(&mut ctx, &admin);
    ctx.tx(&[&admin])
        .build(
            stake_bundle(&admin, &asset, &collection),
            staking::client::args::Stake {},
        )
        .send_ok();

    // Only 1 of the 7 freeze-period days has elapsed.
    ctx.svm.advance_days(1);
    let ix = ctx.program().build_ix(
        unstake_bundle(&admin, &asset, &collection),
        staking::client::args::Unstake {},
    );
    let result = ctx.send_err_named(ix, &[&admin], "FreezePeriodNotElapsed");
    common::expect_capture("stake_freeze_locked", &result.tree_string());
}

/// Same setup, but the clock is warped past the 7-day freeze period before
/// `unstake` runs. The tree shows the mpl-core unfreeze / plugin-update CPIs
/// plus the rewards `mint_to` token CPI that `stake_happy_path`'s tree never
/// reaches.
#[test]
fn unstake_after_freeze_succeeds() {
    let mut ctx = boot();
    let admin = ctx.cast_actor("Alice");
    let (collection, asset) = setup(&mut ctx, &admin);
    ctx.tx(&[&admin])
        .build(
            stake_bundle(&admin, &asset, &collection),
            staking::client::args::Stake {},
        )
        .send_ok();

    // 8 of the 7 freeze-period days have elapsed.
    ctx.svm.advance_days(8);
    let result = ctx
        .tx(&[&admin])
        .build(
            unstake_bundle(&admin, &asset, &collection),
            staking::client::args::Unstake {},
        )
        .send_ok();
    common::expect_capture("stake_unstake_ok", &result.tree_string());
}
