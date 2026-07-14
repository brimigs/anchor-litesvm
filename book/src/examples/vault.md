# Vault

<details>
<summary>Your starting point</summary>

The vault program's full source, a standard Anchor program with no tests, at
`examples/vault/`. Its built `.so` and IDL are committed too, so a fresh clone
runs this chapter's test without building anything:

```bash
git clone -b feat/buildable-ix https://github.com/cds-rs/anchor-litesvm
cd anchor-litesvm
cargo test -p anchor-litesvm --test book_vault
```

```text
examples/vault/                                the program source (no tests)
crates/anchor-litesvm/tests/fixtures/vault.so  the built program
crates/anchor-litesvm/idls/vault.json          its IDL
crates/anchor-litesvm/tests/book_vault.rs      this chapter's test
```

Changed the program? Rebuild the fixture with `cd examples/vault && anchor build`.

</details>

The vault program has four instructions. `initialize` creates a per-user
`vault_state` PDA and its companion `vault` PDA; `deposit` moves lamports in
and emits a `Deposited` event; `withdraw` moves them back out; `close`
returns the rent.

This chapter drives two of those four, `initialize` and `deposit`, through
`anchor-litesvm`. Then it turns to what this book calls the escape hatch: a
way to build an instruction honestly from a bundle and then override exactly
one account slot, so a test can submit the specific malformed transaction an
attacker would send, without hand-assembling every other account itself.
Vault is where that idea gets its first real workout, against an attacker
who tries to substitute someone else's account for her own.

## Boot and deposit

```rust
// crates/anchor-litesvm/tests/book_vault.rs
anchor_lang::declare_program!(vault);
anchor_litesvm::bundles_from_idl!(vault);

fn boot() -> anchor_litesvm::AnchorContext {
    let mut ctx =
        AnchorLiteSVM::build_with_program(vault::ID, "vault", &common::fixture_bytes("vault"));
    // Decode `Deposited` badges from the committed IDL.
    ctx.register_events_from_idl(include_str!("../idls/vault.json"));
    ctx
}
```

`declare_program!` generates the typed client from the vault IDL; without
it, you'd be building instructions by hand, the way the stake chapter does
for a program with no IDL to read. `bundles_from_idl!` then generates an
account bundle (`InitializeBundle`, `DepositBundle`, ...) for each
instruction, deriving PDAs so you only supply the accounts that vary per
call. Here that's just `user`: both `vault_state` and `vault` are PDAs
derivable from it, so the bundle fills them in for you.

`register_events_from_idl` reads that same IDL and registers a decoder for
every event the program declares. That's what makes `result.parse_event()`
below work: without a registered decoder for `Deposited`, there would be
nothing for it to decode the event log line into.

```rust
// crates/anchor-litesvm/tests/book_vault.rs
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

let ev: vault::events::Deposited = result.parse_event().expect("Deposited event present");
assert_eq!(ev.amount, 1_000_000_000);
```

`result.tree_string()` renders the transaction as a CPI tree:

```text
{{#include ../captured/vault_deposit.txt}}
```

`deposit`'s own frame is `[1]`; the `System [2]` child one level deeper is
the lamport transfer `deposit` makes via CPI into `system_program`. The 🔔
line is the decoded `Deposited` event, sitting inside `deposit`'s own frame
since that's where `emit!` was called. `user` prints as `Alice` rather than
a raw pubkey because the decoder resolves pubkey fields through the same
alias table `cast_actor` registered her into.

## The escape hatch

`build_ix` derives every account from the bundle honestly, the same path
`initialize` and `deposit` just took above. `build_ix_with` does the same
derivation, then hands you a closure that overrides exactly one slot
afterward. That one-slot override is the whole trick: it lets a test
construct the specific malformed instruction an attacker would submit,
identical to a legitimate call in every other account and in the
instruction data, without hand-rolling every other account itself.

Mallory wants Alice's deposit. You might wonder why she bothers
initializing her own vault first rather than reusing some other account she
already has lying around. Here's the reason: `Account<'info, VaultState>`
checks its owner and its discriminator before any explicit constraint on
that field runs, so a plainly-wrong account, wrong owner or wrong
discriminator, gets rejected on the spot, before the seeds check downstream
even gets a chance to fire. To get past those two checks, Mallory needs the
substitute to genuinely be a `VaultState` account owned by the vault
program, so she runs her own `initialize` first (not shown in the excerpt
below, since it is identical to Alice's), which gives her exactly that: a
real, program-owned, correctly-discriminated `VaultState` account at *her*
PDA.

Then she submits a deposit into Alice's vault, with the `vault_state` slot
swapped for that account:

```rust
// crates/anchor-litesvm/tests/book_vault.rs
let (mallory_state, _) = vault_state_pda(&mallory.pubkey());
let ix = ctx.program().build_ix_with(
    DepositBundle {
        user: alice.pubkey(),
    },
    vault::client::args::Deposit {
        amount: 1_000_000_000,
    },
    |accounts| accounts.vault_state = mallory_state,
);

let result = ctx.send_err_named(ix, &[&alice], "ConstraintSeeds");
```

```text
{{#include ../captured/vault_wrong_state.txt}}
```

Anchor loads Mallory's account without complaint: right owner, the vault
program; right discriminator, `VaultState`'s own. The `✗` leaf is
`ConstraintSeeds`, though. The field's `seeds` constraint re-derives the
expected PDA from the seeds declared on `vault_state`, which include
`user`'s key, Alice's, since `user` wasn't overridden, and compares that
derivation to the address actually supplied for `vault_state`, Mallory's.
The two don't match, so the constraint rejects the swap.

That's the confused-deputy story: a substituted account can be valid in
every way that matters to the deserializer, right owner, right type, and
still belong to the wrong party. `ConstraintSeeds` is the one check here
that ties this specific field to Alice's key rather than anyone else's, and
it's what catches the substitution.

The full test is `crates/anchor-litesvm/tests/book_vault.rs`.
