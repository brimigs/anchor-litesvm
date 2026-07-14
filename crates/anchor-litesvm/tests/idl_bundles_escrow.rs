//! Build-only: the escrow analog of `idl_bundles.rs`. Escrow is anchor-1.0's
//! richer shape (SPL tokens via `anchor-spl`, `init_if_needed`, a time-lock),
//! so this is also the proof that its IDL ingests cleanly into
//! `declare_program!`/`bundles_from_idl!` at all.
// declare_program!'s expansion gates on-chain-only branches with
// `cfg(target_os = "solana")` and `cfg(feature = "idl-build")`; off-chain those
// compile out, but check-cfg doesn't know the names, so silence the noise here.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::Pubkey;
// `self` binds the crate name into this test's root scope; declare_program!'s
// generated modules reach `anchor_lang` via `super::`, so it must be nameable
// here (without it, the expansion fails with "no `anchor_lang` in the root").
use anchor_lang::{self, InstructionData, ToAccountMetas};

anchor_lang::declare_program!(escrow);
anchor_litesvm::bundles_from_idl!(escrow);

const ASSOCIATED_TOKEN_PROGRAM: Pubkey =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SYSTEM_PROGRAM: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");

// `make`'s Accounts derive names the ix-arg `seeds` (not `seed`, matching the
// vendored source's `#[instruction(seeds: u64)]`), so `escrow`'s PDA seeds an
// arg the emitter can't see at build time: it demotes to a bundle field, same
// as `take`/`refund`'s self-referential `escrow.maker`/`escrow.seed` seeds.
#[test]
fn make_bundle_matches_hand_built_instruction() {
    let maker = Pubkey::new_unique();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    // ArgSeed-demoted: the caller supplies this PDA (or its own fixture address).
    let escrow = Pubkey::new_unique();

    let bundle = MakeBundle {
        maker,
        mint_a,
        mint_b,
        escrow,
        ..Default::default()
    };
    let accounts: escrow::client::accounts::Make = bundle.into();

    let (maker_ata_a, _) = maker_ata_a_pda(&maker, &accounts.token_program, &mint_a);
    let (vault, _) = vault_pda(&escrow, &accounts.token_program, &mint_a);
    assert_eq!(accounts.maker_ata_a, maker_ata_a);
    assert_eq!(accounts.vault, vault);
    assert_eq!(accounts.escrow, escrow, "escrow rides through untouched");

    let hand = escrow::client::accounts::Make {
        maker,
        mint_a,
        mint_b,
        maker_ata_a,
        escrow,
        vault,
        token_program: accounts.token_program,
        associated_token_program: ASSOCIATED_TOKEN_PROGRAM,
        system_program: SYSTEM_PROGRAM,
    };
    assert_eq!(accounts.to_account_metas(None), hand.to_account_metas(None));
}

#[test]
fn take_bundle_matches_hand_built_instruction() {
    let taker = Pubkey::new_unique();
    let maker = Pubkey::new_unique();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    // DataPathSeed-demoted: `escrow`'s own `seeds = [.., escrow.maker,
    // escrow.seed]` constraint reads account data, unresolvable from pubkeys.
    let escrow = Pubkey::new_unique();

    let bundle = TakeBundle {
        taker,
        maker,
        mint_a,
        mint_b,
        escrow,
        ..Default::default()
    };
    let accounts: escrow::client::accounts::Take = bundle.into();

    let (taker_ata_a, _) = taker_ata_a_pda(&taker, &accounts.token_program, &mint_a);
    let (taker_ata_b, _) = taker_ata_b_pda(&taker, &accounts.token_program, &mint_b);
    let (maker_ata_b, _) = maker_ata_b_pda(&maker, &accounts.token_program, &mint_b);
    let (vault, _) = vault_pda(&escrow, &accounts.token_program, &mint_a);

    let hand = escrow::client::accounts::Take {
        taker,
        maker,
        mint_a,
        mint_b,
        taker_ata_a,
        taker_ata_b,
        maker_ata_b,
        escrow,
        vault,
        token_program: accounts.token_program,
        associated_token_program: ASSOCIATED_TOKEN_PROGRAM,
        system_program: SYSTEM_PROGRAM,
    };
    assert_eq!(accounts.to_account_metas(None), hand.to_account_metas(None));
}

#[test]
fn refund_bundle_matches_hand_built_instruction() {
    let maker = Pubkey::new_unique();
    let mint_a = Pubkey::new_unique();
    let escrow = Pubkey::new_unique();

    let bundle = RefundBundle {
        maker,
        mint_a,
        escrow,
        ..Default::default()
    };
    let accounts: escrow::client::accounts::Refund = bundle.into();

    let (maker_ata_a, _) = maker_ata_a_pda(&maker, &accounts.token_program, &mint_a);
    let (vault, _) = vault_pda(&escrow, &accounts.token_program, &mint_a);

    // Refund never touches the associated-token program directly (no `init`
    // on a token account), so unlike `make`/`take` it does not carry that
    // account at all.
    let hand = escrow::client::accounts::Refund {
        maker,
        mint_a,
        maker_ata_a,
        escrow,
        vault,
        token_program: accounts.token_program,
        system_program: SYSTEM_PROGRAM,
    };
    assert_eq!(accounts.to_account_metas(None), hand.to_account_metas(None));
}

// `#[instruction(discriminator = 0)]` on `make` overrides Anchor's default
// 8-byte sighash with a literal 1-byte discriminator; the committed IDL
// carries it as `"discriminator": [0]`, and this is that IDL riding through
// `client::args::Make::data()` unchanged.
#[test]
fn make_discriminator_is_the_explicit_single_byte_zero() {
    let args = escrow::client::args::Make {
        seed: 1,
        receive: 2,
        deposit: 3,
    };
    let data = args.data();
    assert_eq!(
        data[0], 0,
        "make's explicit discriminator is the single byte 0"
    );

    let mut expected = vec![0u8];
    expected.extend_from_slice(&1u64.to_le_bytes());
    expected.extend_from_slice(&2u64.to_le_bytes());
    expected.extend_from_slice(&3u64.to_le_bytes());
    assert_eq!(data, expected);
}

#[test]
fn injected_programs_lists_the_associated_token_and_system_programs() {
    let injected = injected_programs();
    assert!(injected.iter().any(
        |(addr, name)| *addr == ASSOCIATED_TOKEN_PROGRAM && *name == "associated_token_program"
    ));
    assert!(injected
        .iter()
        .any(|(addr, name)| *addr == SYSTEM_PROGRAM && *name == "system_program"));
}
