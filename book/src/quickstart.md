# Quickstart

Deploy an Anchor program, deposit into it, and print the structured logs:
that's the fastest path to a passing test.

See the [Vault chapter](examples/vault.md) for the same program driven in depth.

## Get the code

The programs and tests live in one repo, whose workspace already wires up
anchor-litesvm and the litesvm fork it builds on. Until those land upstream,
clone the fork and work from inside it:

```bash
git clone -b feat/buildable-ix https://github.com/cds-rs/anchor-litesvm
cd anchor-litesvm
cargo test -p anchor-litesvm --test book_vault
```

There is nothing to add to a `Cargo.toml`: the workspace resolves every
dependency. That command runs the vault chapter's test against the committed
`vault.so` fixture, and the rest of this page is the code it runs.

## The test

Three things below are worth naming before you hit them, since the code leans
on all three without pausing to explain itself.

`ctx.cast_actor("Alice")` casts an actor: a funded, named keypair standing in
for a real signer. From here on, the test (and the printed logs) refer to her
as `Alice` instead of a 44-character pubkey.

The comment about PDAs refers to Program Derived Addresses: account addresses
derived deterministically from seeds instead of from a keypair, which is how
a program can own and sign for `vault_state` and `vault` below without anyone
holding a private key for them.

And `InitializeBundle` / `DepositBundle` are bundles: generated structs that
hold exactly the accounts a given instruction's caller needs to supply, PDAs
already derived, so you only fill in what actually varies per call (here,
just `user`).

```rust
#![allow(unexpected_cfgs)]

use anchor_lang::{self};
use anchor_litesvm::{AnchorLiteSVM, Signer};

anchor_lang::declare_program!(vault);
anchor_litesvm::bundles_from_idl!(vault);

#[test]
fn deposit_happy_path() {
    // vault.so is the committed fixture; book_vault.rs loads the same bytes
    // with common::fixture_bytes("vault").
    let mut ctx = AnchorLiteSVM::build_with_program(
        vault::ID,
        "vault",
        include_bytes!("fixtures/vault.so"),
    );
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

    // deposit 1 SOL.
    ctx.tx(&[&alice])
        .build(
            DepositBundle {
                user: alice.pubkey(),
            },
            vault::client::args::Deposit {
                amount: 1_000_000_000,
            },
        )
        .send_ok()
        .print_logs();
}
```

`declare_program!` (from `anchor_lang`) is the macro that generates the typed
client from the vault IDL. `bundles_from_idl!` (from `anchor_litesvm`) is the
one that generates the bundles you just met above, one per instruction.

`print_logs()` renders the transaction as a CPI tree, program logs and all:
the same format introduced above, printed instead of just returned.

Next: the [Vault chapter](examples/vault.md) drives the same program deeper
(events, the escape hatch, a rejected transaction).

[Concepts](concepts/aliases.md) covers aliases, the World, and structured logs
from here, in more depth than a quickstart has room for.
