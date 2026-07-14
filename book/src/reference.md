# Reference

A curated "which tool and why," organized by the object you hold. Think of
it as a map, not a signature dump: docs.rs already has the exact
signatures, so this page has a narrower job. It tells you which method to
reach for, and points at the example chapter that puts it to work.

## `AnchorLiteSVM` (the builder)

This is the first line of every test: deploy the program(s), get back a
ready `AnchorContext`.

- **`build_with_program(id, name, &bytes)`**: deploys one program. `name`
  gets registered as an alias, so when a tree fails, it names the program
  instead of showing you its raw pubkey. Used throughout
  [Vault](examples/vault.md) and [Escrow](examples/escrow.md).
- **`build_with_programs(&[(id, name, &bytes), ...])`**: deploys several
  programs in one call, aliasing each. Reach for this when the program
  under test CPIs into another one your test must also deploy, as
  [Stake](examples/stake.md) does for `mpl-core`. The first entry becomes
  the context's primary `program_id`.

## `AnchorContext` (the World)

This is the cast-and-setup surface. See [The World](concepts/world.md) and
[Setup](concepts/setup.md) for the narrative version of the same ideas.

- **`cast_actor(name)`**: a deterministic, 100-SOL-funded, aliased signer.
  The default way to bring an actor ("Alice", "Bob", "Mallory") into a
  scenario. See [Aliases & Actors](concepts/aliases.md).
- **`cast_actor_with_sol(name, lamports)`**: same as `cast_actor`, with an
  explicit lamport balance instead of the 100 SOL default. Reach for it
  when a scenario asserts on an exact SOL amount.
- **`cast_mint(name, &authority, decimals)`**: casts a token mint under
  `authority`, aliased `name`. Used throughout [Escrow](examples/escrow.md)
  (`MintA`, `MintB`).
- **`fund_ata(&owner, &mint, &authority, amount)`**: creates `owner`'s
  associated token account for `mint`, mints `amount` into it, and aliases
  the ATA `"<owner>/<mint>"`. This is the funded-holder setup Escrow uses
  for both sides of the trade.
- **`alias(pubkey, name)`** / **`alias_ata(&owner, &mint)`**: register a
  pubkey (or a derived ATA) under a name directly, for accounts that
  didn't come from a `cast_*` call.
- **`register_events_from_idl(idl_json)`**: registers a decoder for every
  event an Anchor IDL declares, so `emit!`ed logs decode into typed values
  and render as `🔔` badges. Vault's `Deposited` event uses this (see
  [Vault](examples/vault.md)).
- **`register_program_errors(program_id, &[(code, name), ...])`**: names a
  program's custom error codes by hand. This is the tool for programs
  without an IDL: Stake's `mpl-core`-dependent program can't feed
  `declare_program!`, so this is the only way a failing leaf reads
  `FreezePeriodNotElapsed` instead of `custom program error: 0x1770` (see
  [Stake](examples/stake.md)).
- **`tx(signers)`**: starts the fluent build-and-send chain. See
  [Sending](#sending) below.
- **`load(&address)`** / **`try_load(&address)`**: deserialize an Anchor
  account at `address`. `load` panics on failure (missing account, wrong
  discriminator (the 8-byte type tag Anchor writes at the front of an
  account's data, so it can tell one account type from another), a deser
  error); that's the idiomatic choice in a test, where the failure itself
  is the test failing. `try_load` returns a `Result` instead, for callers
  that want to handle it themselves. `load_unchecked` /
  `try_load_unchecked` skip that discriminator check, for the rare case
  where you need the raw bytes regardless of what type tag they carry.

## Sending

Every send method asserts something specific, so a glance at the call name
alone tells a reader what the test expects:

| Method | Asserts | Reach for it when |
|---|---|---|
| `send_ok(ix, signers)` | transaction succeeds | the happy path |
| `send_err(ix, signers)` | transaction fails, any error | the outcome alone is the contract (an authorization check, a generic constraint trip) and pinning to a specific error name would over-constrain the test |
| `send_err_named(ix, signers, "Name")` | transaction fails *and* the failure resolves to (or its logs contain) `"Name"` | you know exactly which error should fire, e.g. `"EscrowExpired"`, `"ConstraintSeeds"` |

All three live on `AnchorContext` two ways: as one-shot calls
(`ctx.send_ok(ix, &[&signer])`), and as the fluent chain's terminators:

```rust
ctx.tx(&[&signer])
   .build(bundle, args)
   .send_ok()
   .print_logs();
```

The `Tx` chain (`ctx.tx(signers).build(bundle, args)`) earns its keep once
a test builds and sends several instructions: `build`/`build_with` share
the same terminators, and `remaining_accounts` appends a dynamic account
tail.

For a single one-off send, though, the one-shot `ctx.send_ok(ix, signers)`
skips the chain entirely. Both return the same `TransactionResult`. See
[Structured Logs](concepts/structured-logs.md#sending-send_ok--send_err--send_err_named).

## Building instructions

Both methods below take a bundle: a struct that groups the accounts one
instruction needs, most of them defaulted so you only bind the ones your
scenario cares about (the full story is in
[Bundle defaults & partial binding](#bundle-defaults--partial-binding)
below). The choice between them is about whether every account should
come straight from the bundle, or whether you need to swap exactly one of
them out.

- **`program().build_ix(bundle, args)`**: derives every account from the
  bundle, no overrides. This is the path every happy-path call in this
  book uses.
- **`program().build_ix_with(bundle, args, |accounts| ...)`**: same
  derivation, plus a closure to override exactly one account before it's
  sent. This is the negative-path escape hatch: it constructs the
  instruction an attacker would submit (a valid-but-wrong account swapped
  in) without making you hand-roll every other account yourself. See the
  [Vault escape hatch](examples/vault.md#the-escape-hatch) (`vault_state`
  swapped for Mallory's own) and the
  [Escrow escape hatch](examples/escrow.md#the-escape-hatch) (`vault`
  swapped for Mallory's ATA).

`bundles_from_idl!(program_name)` reads a committed IDL and writes that
bundle machinery for you. Two kinds of account never need to be named by
a caller at all: a PDA (a Program Derived Address, one derivable from
known seeds) gets computed on the spot, and a fixed program address the
IDL pins (the token program, a specific CPI target) gets filled in
automatically. This reference calls that second behavior injecting the
account, since the caller never supplies it and never sees it as a bundle
field.

Concretely, the macro generates one `<Ix>Bundle` struct per instruction (a
plain-pubkey field only for the accounts a caller must actually supply), a
`<account>_pda(...)` helper per derivable PDA, and a module-level
`injected_programs()` listing every address it injects this way. The
bundle's `From` impl is what performs both the derivation and the
injection, so `build_ix` only ever needs the accounts that vary per call.
See [Setup](concepts/setup.md) and the [Quickstart](quickstart.md).

When the program's IDL can't feed `declare_program!` / `bundles_from_idl!`
at all (the anchor-version wall: [Stake](examples/stake.md)'s dependency
on `mpl-core` pins it to anchor 0.31, while the host workspace is anchor
1.0), drop to hand-built `solana_instruction::Instruction`s instead:
compute the 8-byte discriminator as `sha256("global:<name>")[..8]`, list
account metas in the program's `#[derive(Accounts)]` order, and derive
PDAs by hand with `find_program_address`. That is exactly what
`bundles_from_idl!` generates for you when an IDL is available.

## Bundle defaults & partial binding

A bundle is a collection of the accounts an instruction needs, with sane
defaults. Bind the accounts your scenario is actually about; let the rest
default. Every generated bundle implements `Default`, so struct-update
syntax binds only what matters:

```rust
// bind the roots this scenario touches; Default covers the rest.
let bundle = MakeBundle { maker, mint_a, mint_b, escrow, ..Default::default() };
```

What `Default` fills:

- **Derivable PDAs and fixed program addresses** never appear as bundle
  fields at all; the `From` impl derives and injects them from the IDL.
- **Caller-supplied accounts** you leave unbound fill with
  `Pubkey::new_unique()`.
- **Token-program fields** fill with their well-known programs
  (`token_program` with classic SPL, `associated_token_program` with the
  ATA program). These are overridable defaults, not injections: a
  scenario on a different token program overrides the field directly, and
  the generated rustdoc on each such field says so.
- **Optional accounts** fill with `None`.

The unbound placeholder is `Pubkey::new_unique()`, not `Pubkey::default()`
(the zero address), and that's deliberate. The zero address gets rejected
by nearly every program before anything interesting runs, while a fresh
unique address behaves as an account that simply does not exist.

That is exactly the probe a partially-bound negative test wants: leave a
field unbound to assert the program rejects a missing account. This is
the complement to [`build_ix_with`](#building-instructions), which swaps
in a valid-but-wrong account rather than an absent one.

## Reading output

- **`print_logs()`**: prints the run's CPI tree to stdout, returns `self`
  so it chains at the end of a call. Reach for this when a human (you,
  mid-debug) is the reader.
- **`tree_string()`**: same rendering, returned as a `String` instead of
  printed. Reach for this when the reader is code instead: asserting
  against the tree, or capturing it as a fixture. Every captured `.txt`
  fixture in this book is a `tree_string()` capture, verbatim. See
  [Structured Logs](concepts/structured-logs.md) for the full anatomy.
- **`EventHelpers::parse_event::<T>()`** / **`parse_events::<T>()`**:
  deserialize a specific Anchor event type out of the logs (the first
  one, or all of them). Reach for these when the test needs the value
  itself, the way Vault's deposit test checks `ev.amount` (see
  [Vault](examples/vault.md)).
- **`assert_event_emitted::<T>()`** / **`assert_event_count::<T>(n)`**:
  assert an event of type `T` was (or wasn't, or was emitted exactly `n`
  times) without pulling the value out yourself. Reach for these instead
  of `parse_event` when firing at all (or firing the right number of
  times) is the whole assertion, and the payload doesn't matter.

## Clock & state helpers (`TestHelpers`)

Methods on `ctx.svm` (a `LiteSVM`), for time-locked program logic and
token state:

- **`advance_days(n)`** / **`advance_seconds(n)`**: move the Clock
  sysvar's `unix_timestamp` forward. This is the tool for time-locked
  constraints: Escrow's 90-day expiry and Stake's 7-day freeze period both
  drive their negative and positive paths by advancing past (or short of)
  the deadline. See [Escrow's time-lock](examples/escrow.md#time-lock) and
  [Stake's freeze lock](examples/stake.md#freeze-lock).
- **`warp_to_timestamp(unix_timestamp)`**: set the Clock's
  `unix_timestamp` to an absolute value, when a test needs a deterministic
  wall-clock point rather than a relative jump.
- **`token_balance(&ata)`**: read an SPL Token account's amount; `None` if
  no account exists there (so a post-close assertion reads `is_none()`
  rather than panicking).
- **`create_token_mint(&authority, decimals)`**: create and initialize a
  token mint with a freshly generated keypair. `cast_mint` (on
  `AnchorContext`) is the aliased, deterministic-keypair wrapper most
  scenarios reach for instead; use this directly when you don't need
  either.
