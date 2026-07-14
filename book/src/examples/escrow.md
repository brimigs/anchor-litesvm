# Escrow

<details>
<summary>Your starting point</summary>

The escrow program's full source, a standard Anchor program with no tests, at
`examples/escrow/`. Its built `.so` and IDL are committed too, so a fresh clone
runs this chapter's test without building anything:

```bash
git clone -b feat/buildable-ix https://github.com/cds-rs/anchor-litesvm
cd anchor-litesvm
cargo test -p anchor-litesvm --test book_escrow
```

```text
examples/escrow/                                the program source (no tests)
crates/anchor-litesvm/tests/fixtures/escrow.so  the built program
crates/anchor-litesvm/idls/escrow.json          its IDL
crates/anchor-litesvm/tests/book_escrow.rs      this chapter's test
```

Changed the program? Rebuild the fixture with `cd examples/escrow && anchor build`.

</details>

The escrow program has three instructions. `make` creates an escrow PDA and
deposits `mint_a` into its vault; `take` lets a counterparty pay `mint_b` and
receive the vault's `mint_a`; `refund` returns the deposit to the maker.

Every escrow carries a 90-day expiry, and `take` and `refund` sit on
opposite sides of it: `take` stops working once the expiry passes, `refund`
only starts working once it does. The Time-lock section below drives both
sides of that boundary.

`make` and `take` also drive real SPL Token CPIs, transfers and
`init_if_needed` associated-token-account creation, which makes this chapter
a good place to read a multi-CPI tree once tokens are involved. This chapter
drives all three instructions through `anchor-litesvm`.

## Boot and make -> take

```rust
// crates/anchor-litesvm/tests/book_escrow.rs
fn boot() -> anchor_litesvm::AnchorContext {
    AnchorLiteSVM::build_with_program(escrow::ID, "escrow", &common::fixture_bytes("escrow"))
}
```

One heads-up before the listing: `bundles_from_idl!` cannot derive every
account for `make` and `take`. One field, `escrow`, needs computing by hand
and passing in like any other bundle value; the code comment marks where,
and the paragraph right after the listing explains why.

```rust
// crates/anchor-litesvm/tests/book_escrow.rs
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
```

`bundles_from_idl!` derives most of `make` and `take`'s accounts, the vault,
the ATAs, the escrow PDA itself, where it can. `escrow` is the one
exception, and the code comment above flags why: the instruction takes an
argument named `seed`, but the IDL's own emitted seed path names that same
argument `seeds`, a vendored-source quirk in how the IDL was generated. The
macro matches a seed path's arguments back to the instruction's by name, so
a name that doesn't line up can't be resolved automatically, and `escrow`
gets demoted from a derived field to a plain one. The caller computes
`escrow_pda` with `find_program_address` instead, and passes it in like any
other bundle field.

`result.tree_string()` renders the transaction as a CPI tree:

```text
{{#include ../captured/escrow_take.txt}}
```

The two `AssociatedToken` frames are `init_if_needed` creating the taker's
and maker's associated token accounts: `init_if_needed` means the
constraint creates the account only if it doesn't already exist, and does
nothing if it does. Nested inside each `AssociatedToken` frame, the
`Token`/`System` frames are the ATA program's own calls to size, fund, and
initialize that account.

The three `Token` frames after that are `take`'s own CPIs, and they run in
the order the instruction issues them: Bob pays Alice `mint_b`, the vault
pays Bob `mint_a`, and the vault account closes, rent back to Alice.

## Time-lock

`litesvm_utils::TestHelpers::advance_days` warps the SVM clock forward by a
given number of days, which is how this section gets to the far side of the
90-day expiry without waiting for it in real time. Push past that expiry and
`take` gets rejected before either ATA transfer happens:

```rust
// crates/anchor-litesvm/tests/book_escrow.rs
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

// The escrow expires 90 days after make. Jump 91 days forward.
ctx.svm.advance_days(91);

let result = ctx.send_err_named(ix, &[&taker], "EscrowExpired");
```

```text
{{#include ../captured/escrow_expired.txt}}
```

Both `AssociatedToken` frames still run and succeed: `init_if_needed` only
checks whether the account already exists, nothing about the escrow's
expiry, so account creation goes ahead regardless. The `✗` leaf is the
program's own expiry check, `EscrowExpired`, and it runs after both ATAs are
already created, which is why the transaction fails only at that point
rather than upfront.

Refund is the mirror image of the same expiry check: it only works *after*
the 90 days pass, so calling it while still inside the window is rejected
too:

```rust
// crates/anchor-litesvm/tests/book_escrow.rs
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
```

```text
{{#include ../captured/escrow_refund_too_early.txt}}
```

Same shape as the expiry check above, mirrored: the program's own
`EscrowNotExpired` guard rejects the call because the 90 days haven't
elapsed yet, and this time there's no ATA creation racing ahead of it,
since `refund` doesn't touch the associated-token-account machinery at all.

## The escape hatch

`build_ix_with` builds every account honestly, then hands you a closure to
override exactly one slot, the same escape hatch the vault chapter used
against `vault_state`.

Mallory wants Bob's `take` to pay out to her instead of the vault. The
swapped account can't be just anything, though: the `vault` field is an
`InterfaceAccount<'info, TokenAccount>`, which checks that the account is
owned by a token program and that its data actually unpacks as an
initialized token account, before any `#[account(...)]` constraint on that
field runs. So Mallory's setup is to initialize her own, genuinely-owned
`mint_a` associated token account first, a real token account that passes
those checks cleanly, then submit `take` with the `vault` slot pointed at
it:

```rust
// crates/anchor-litesvm/tests/book_escrow.rs
// Mallory owns a real, initialized mint_a token account (the
// confused-deputy setup: valid in every way except its authority is
// Mallory, not the escrow PDA). Zero balance is fine; it only needs to
// exist and deserialize. `maker` is the mint authority as elsewhere.
let mallory_vault = ctx.fund_ata(&mallory, &mint_a, &maker, 0);

// Point vault at Mallory's ATA instead of the escrow PDA's. The bundle
// derives every account honestly; the closure then swaps exactly the
// vault slot.
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

let result = ctx.send_err_named(ix, &[&taker], "ConstraintTokenOwner");
```

```text
{{#include ../captured/escrow_wrong_vault.txt}}
```

Mallory's ATA deserializes fine: real mint, real token account, right
discriminator. Both `AssociatedToken` frames still succeed, same as the
happy path, so nothing about account creation flags the substitution.

What catches it is `vault`'s own `associated_token::authority = escrow`
constraint, checked once the account is already loaded: it reads the token
account's actual owner field and compares it to the escrow PDA. Mallory's
ATA is owned by Mallory, not by escrow, so the two don't match and Anchor
rejects with `ConstraintTokenOwner`.

Same confused-deputy lesson as the vault chapter's `ConstraintSeeds`: a
substituted account can be valid in every way that matters to the
deserializer, and still belong to the wrong party. Here, as there, it's one
constraint, checked after the account is already loaded, that ties the
field to the right owner and catches the swap.

The full test is `crates/anchor-litesvm/tests/book_escrow.rs`.
