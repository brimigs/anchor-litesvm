//! Book capture: deploy the escrow program, run make + take as Alice/Bob,
//! and snapshot the rendered CPI tree (the SPL Token transfers nested under
//! `escrow::take`) into `book/src/captured/escrow_take.txt`.
// declare_program!'s expansion gates on-chain-only branches with
// `cfg(target_os = "solana")` and `cfg(feature = "idl-build")`; off-chain those
// compile out, but check-cfg doesn't know the names, so silence the noise here.
#![allow(unexpected_cfgs)]
mod common;

// `self` binds the crate name into this test's root scope; declare_program!'s
// generated modules reach `anchor_lang` via `super::`, so it must be nameable
// here (without it, the expansion fails with "no `anchor_lang` in the root").
use anchor_lang::prelude::Pubkey;
use anchor_lang::{self};
use anchor_litesvm::{AnchorLiteSVM, Signer};
use litesvm_utils::TestHelpers;

anchor_lang::declare_program!(escrow);
anchor_litesvm::bundles_from_idl!(escrow);

// escrow's Cargo graph doesn't reach `anchor-spl`, so there's no `anchor_spl::
// token::ID` to borrow; the classic SPL Token program id is a well-known
// constant instead (the mints `cast_mint`/`fund_ata` create are classic SPL,
// not Token-2022).
const TOKEN_PROGRAM: Pubkey = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

fn boot() -> anchor_litesvm::AnchorContext {
    AnchorLiteSVM::build_with_program(escrow::ID, "escrow", &common::fixture_bytes("escrow"))
}

#[test]
fn escrow_make_then_take() {
    let mut ctx = boot();
    let maker = ctx.cast_actor("Alice"); // Alice makes the escrow
    let taker = ctx.cast_actor("Bob"); // Bob takes it
    let mint_a = ctx.cast_mint("MintA", &maker, 6);
    let mint_b = ctx.cast_mint("MintB", &maker, 6);

    // Fund Alice with MintA (offered) and Bob with MintB (wanted).
    let _alice_a = ctx.fund_ata(&maker, &mint_a, &maker, 1_000_000);
    let _bob_b = ctx.fund_ata(&taker, &mint_b, &maker, 1_000_000);

    // `escrow`'s PDA seeds an ix-arg (`seed`) that the IDL's own emitted
    // seed-path names `seeds` (a vendored-source quirk), so the macro can't
    // resolve it at build time and demotes it to a plain bundle field: the
    // caller derives and supplies it directly, here and again in `take`.
    let seed = 42u64;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::ID,
    );

    ctx.tx(&[&maker])
        .build(
            MakeBundle {
                maker: maker.pubkey(),
                mint_a,
                mint_b,
                token_program: TOKEN_PROGRAM,
                escrow: escrow_pda,
            },
            escrow::client::args::Make {
                seed,
                receive: 1_000_000,
                deposit: 1_000_000,
            },
        )
        .send_ok();

    // take: Bob pays MintB to Alice and receives MintA from the vault.
    let result = ctx
        .tx(&[&taker])
        .build(
            TakeBundle {
                taker: taker.pubkey(),
                maker: maker.pubkey(),
                mint_a,
                mint_b,
                token_program: TOKEN_PROGRAM,
                escrow: escrow_pda,
            },
            escrow::client::args::Take {},
        )
        .send_ok();

    // Bob now holds the offered MintA. The vendored escrow swallows some CPI
    // `Result`s (`let _ = transfer_checked(...)`), so a silently-failed
    // transfer would show up as a wrong balance here: this assertion is the
    // real guard, not the `send_ok()` above.
    let bob_a = ctx.alias_ata(&taker.pubkey(), &mint_a);
    assert_eq!(ctx.svm.token_balance(&bob_a), Some(1_000_000));

    common::expect_capture("escrow_take", &result.tree_string());
}

#[test]
fn take_after_expiry_is_rejected() {
    let mut ctx = boot();
    let maker = ctx.cast_actor("Alice");
    let taker = ctx.cast_actor("Bob");
    let mint_a = ctx.cast_mint("MintA", &maker, 6);
    let mint_b = ctx.cast_mint("MintB", &maker, 6);
    ctx.fund_ata(&maker, &mint_a, &maker, 1_000_000);
    ctx.fund_ata(&taker, &mint_b, &maker, 1_000_000);

    let seed = 7u64;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::ID,
    );

    ctx.tx(&[&maker])
        .build(
            MakeBundle {
                maker: maker.pubkey(),
                mint_a,
                mint_b,
                token_program: TOKEN_PROGRAM,
                escrow: escrow_pda,
            },
            escrow::client::args::Make {
                seed,
                receive: 1_000_000,
                deposit: 1_000_000,
            },
        )
        .send_ok();

    // The escrow expires 90 days after make. Jump 91 days forward.
    ctx.svm.advance_days(91);

    let ix = ctx.program().build_ix(
        TakeBundle {
            taker: taker.pubkey(),
            maker: maker.pubkey(),
            mint_a,
            mint_b,
            token_program: TOKEN_PROGRAM,
            escrow: escrow_pda,
        },
        escrow::client::args::Take {},
    );
    let result = ctx.send_err_named(ix, &[&taker], "EscrowExpired");
    common::expect_capture("escrow_expired", &result.tree_string());
}

#[test]
fn refund_before_expiry_is_rejected() {
    let mut ctx = boot();
    let maker = ctx.cast_actor("Alice");
    let mint_a = ctx.cast_mint("MintA", &maker, 6);
    let mint_b = ctx.cast_mint("MintB", &maker, 6);
    ctx.fund_ata(&maker, &mint_a, &maker, 1_000_000);

    let seed = 9u64;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::ID,
    );

    ctx.tx(&[&maker])
        .build(
            MakeBundle {
                maker: maker.pubkey(),
                mint_a,
                mint_b,
                token_program: TOKEN_PROGRAM,
                escrow: escrow_pda,
            },
            escrow::client::args::Make {
                seed,
                receive: 1_000_000,
                deposit: 1_000_000,
            },
        )
        .send_ok();

    // No time warp: still inside the 90-day window, so refund must be rejected.
    // `refund` doesn't sign with `maker` (it's a plain `SystemAccount`), but the
    // transaction still needs a fee-payer signer, so `maker` signs in that role.
    let ix = ctx.program().build_ix(
        RefundBundle {
            maker: maker.pubkey(),
            mint_a,
            token_program: TOKEN_PROGRAM,
            escrow: escrow_pda,
        },
        escrow::client::args::Refund {},
    );
    let result = ctx.send_err_named(ix, &[&maker], "EscrowNotExpired");
    common::expect_capture("escrow_refund_too_early", &result.tree_string());
}

#[test]
fn take_with_wrong_vault_is_rejected() {
    let mut ctx = boot();
    let maker = ctx.cast_actor("Alice");
    let taker = ctx.cast_actor("Bob");
    let mallory = ctx.cast_actor("Mallory");
    let mint_a = ctx.cast_mint("MintA", &maker, 6);
    let mint_b = ctx.cast_mint("MintB", &maker, 6);
    ctx.fund_ata(&maker, &mint_a, &maker, 1_000_000);
    ctx.fund_ata(&taker, &mint_b, &maker, 1_000_000);

    let seed = 11u64;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::ID,
    );

    ctx.tx(&[&maker])
        .build(
            MakeBundle {
                maker: maker.pubkey(),
                mint_a,
                mint_b,
                token_program: TOKEN_PROGRAM,
                escrow: escrow_pda,
            },
            escrow::client::args::Make {
                seed,
                receive: 1_000_000,
                deposit: 1_000_000,
            },
        )
        .send_ok();

    // Mallory owns a real, initialized mint_a token account (the
    // confused-deputy setup: valid in every way except its authority is
    // Mallory, not the escrow PDA). Zero balance is fine; it only needs to
    // exist and deserialize. `maker` is the mint authority as elsewhere.
    let mallory_vault = ctx.fund_ata(&mallory, &mint_a, &maker, 0);

    // Point vault at Mallory's ATA instead of the escrow PDA's. The bundle
    // derives every account honestly; the closure then swaps exactly the
    // vault slot.
    let honest = ctx.program().build_ix(
        TakeBundle {
            taker: taker.pubkey(),
            maker: maker.pubkey(),
            mint_a,
            mint_b,
            token_program: TOKEN_PROGRAM,
            escrow: escrow_pda,
        },
        escrow::client::args::Take {},
    );
    let ix = ctx.program().build_ix_with(
        TakeBundle {
            taker: taker.pubkey(),
            maker: maker.pubkey(),
            mint_a,
            mint_b,
            token_program: TOKEN_PROGRAM,
            escrow: escrow_pda,
        },
        escrow::client::args::Take {},
        |accounts| accounts.vault = mallory_vault,
    );

    // Prove the mechanism: exactly one account slot differs from the honest
    // build, and it's the corrupted vault.
    let diffs: Vec<usize> = honest
        .accounts
        .iter()
        .zip(&ix.accounts)
        .enumerate()
        .filter(|(_, (a, b))| a.pubkey != b.pubkey)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(diffs.len(), 1, "exactly one slot corrupted");
    assert_eq!(ix.accounts[diffs[0]].pubkey, mallory_vault);

    // Mallory's ATA deserializes fine (real, initialized, mint_a), so Anchor
    // reaches `vault`'s `associated_token::authority = escrow` constraint,
    // which catches that the token owner is Mallory, not the escrow PDA:
    // ConstraintTokenOwner.
    let result = ctx.send_err_named(ix, &[&taker], "ConstraintTokenOwner");
    common::expect_capture("escrow_wrong_vault", &result.tree_string());
}
