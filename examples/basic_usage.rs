//! Quick-start: pair `declare_program!` with `bundles_from_idl!` to turn a
//! committed Anchor IDL into a typed instruction builder, then drive it
//! through an `AnchorContext`.
//!
//! This file is a browsing copy of `crates/anchor-litesvm/examples/basic_usage.rs`,
//! the real, compiling example (`cargo run -p anchor-litesvm --example basic_usage`).
//! There's no crate at the repo root, so this copy isn't itself a cargo target.
//!
//! `declare_program!`'s expansion gates a couple of on-chain-only branches
//! behind `cfg(target_os = "solana")` / `cfg(feature = "idl-build")`; those
//! compile out here (we're off-chain), but `check-cfg` doesn't know the
//! names, hence the silenced lint below.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::Pubkey;
// clippy sees this as redundant because the 2018+ extern prelude already
// makes `anchor_lang::` resolvable here; but `declare_program!`'s generated
// modules reach the crate via `super::anchor_lang`, which only sees names
// bound by an actual `use` in this module, not the extern-prelude fallback.
#[allow(clippy::single_component_path_imports)]
use anchor_lang;
use anchor_litesvm::AnchorContext;
use litesvm::LiteSVM;

// 1. Generate `vault::client::{accounts, args}` from the committed IDL...
anchor_lang::declare_program!(vault);
// 2. ...and pair them with a caller-facing pubkey bundle per instruction,
//    generated from that same IDL.
anchor_litesvm::bundles_from_idl!(vault);

fn main() {
    // Normally: AnchorLiteSVM::build_with_program(vault::ID, "vault",
    // include_bytes!("../target/deploy/vault.so")). No compiled program
    // ships in this repo, so this example builds the instruction without
    // deploying or executing it.
    let ctx = AnchorContext::new(LiteSVM::new(), vault::ID);

    let user = Pubkey::new_unique();

    // 3. `DepositBundle` has exactly one field: `user`. The `vault` and
    //    `vault_state` PDAs, and `system_program`, are derived/injected by
    //    the bundle's generated `From` impl.
    let ix = ctx.program().build_ix(
        DepositBundle { user },
        vault::client::args::Deposit { amount: 1_000_000 },
    );

    assert_eq!(ix.accounts.len(), 4);
    println!("built a `deposit` instruction for {} accounts", ix.accounts.len());

    // 4. The PDA helpers recompute the whole seed chain from the root
    //    account alone, so a test can assert on them directly. Account
    //    order follows the IDL: user, vault, vault_state, system_program.
    let (vault, _bump) = vault_pda(&user);
    let (vault_state, _bump) = vault_state_pda(&user);
    assert_eq!(ix.accounts[1].pubkey, vault);
    assert_eq!(ix.accounts[2].pubkey, vault_state);
    println!("vault PDA: {vault}, vault_state PDA: {vault_state}");
}
