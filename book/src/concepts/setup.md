# Setup

Setup has two jobs: get the program(s) onto the SVM, then populate the
World's cast and event registry, so the rest of the test can talk about
actors and events instead of raw accounts and log bytes.

## Deploying

`AnchorLiteSVM::build_with_program` deploys a single program and returns a
ready `AnchorContext`:

```rust
let mut ctx =
    AnchorLiteSVM::build_with_program(vault::ID, "vault", &common::fixture_bytes("vault"));
```

`program_bytes` is a compiled `.so`'s contents, typically a committed
fixture loaded with `include_bytes!("../fixtures/vault.so")`. Using a
committed `.so` rather than compiling the program as part of the test is
what keeps the test fast and independent of the program crate's own build
toolchain.

The `"vault"` name is registered as an alias for `vault::ID`, so a failing
transaction's tree names the program `vault` instead of its raw pubkey, the
same alias-table mechanism [Aliases & Actors](aliases.md) covers for actors.

When a program CPIs into another one your test must also deploy (the stake
example calls into `mpl-core`), use `build_with_programs` instead, which
takes a list and aliases each entry the same way:

```rust
let mut ctx = AnchorLiteSVM::build_with_programs(&[
    (STAKING_ID, "staking", &common::fixture_bytes("staking")),
    (MPL_CORE_ID, "mpl_core", &common::fixture_bytes("mpl_core")),
]);
```

The first program passed becomes the context's primary `program_id`.

## Casting the scenario

With the context built, cast the actors and accounts the scenario needs. The
vault chapter's full setup:

```rust
fn boot() -> anchor_litesvm::AnchorContext {
    let mut ctx =
        AnchorLiteSVM::build_with_program(vault::ID, "vault", &common::fixture_bytes("vault"));
    // Decode `Deposited` badges from the committed IDL.
    ctx.register_events_from_idl(include_str!("../idls/vault.json"));
    ctx
}
```

```rust
let mut ctx = boot();
let alice = ctx.cast_actor("Alice");
```

`cast_actor(name)` casts a funded, aliased signer, as covered in
[Aliases & Actors](aliases.md).

Two more cast methods round out the vocabulary for token scenarios (used
throughout the escrow example):

- `cast_mint(name, &authority, decimals)` casts a token mint under
  `authority`, aliased `name`.
- `fund_ata(&owner, &mint, &authority, amount)` creates `owner`'s associated
  token account for `mint`, mints `amount` into it from `authority`, and
  aliases the ATA `"<owner>/<mint>"`.

`register_events_from_idl(idl_json)` reads an Anchor IDL (embedded with
`include_str!` so it travels with the test binary) and registers a decoder
for every event it declares.

From then on, any `emit!`ed event in the program's logs decodes into a typed
value (`result.parse_event::<T>()`) and renders as a `🔔` badge in the
printed tree, covered next in [Structured Logs](structured-logs.md).

For a program with no IDL (the stake example's hand-built `.so`), there's
nothing for `register_events_from_idl` to read, so errors need a different
path: `register_program_errors` is the error-side equivalent, naming custom
error codes directly by hand instead of reading them out of an IDL.
