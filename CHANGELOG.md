# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `litesvm-utils`: four timestamp-based helpers on the `TestHelpers` trait for
  testing time-locked logic (escrow expiries, vesting cliffs, etc.) without
  manually round-tripping the `Clock` sysvar:
  - `get_unix_timestamp()` reads the Clock sysvar's `unix_timestamp`
  - `warp_to_timestamp(unix_ts)` sets `unix_timestamp` to an absolute value
  - `advance_seconds(seconds)` advances `unix_timestamp` by the given seconds
  - `advance_days(days)` convenience wrapper over `advance_seconds`

  Other Clock fields (slot, epoch, etc.) are left unchanged; for slot-based
  warping, continue using `advance_slot` / `warp_to_slot`.

- `litesvm-utils`: a naming vocabulary shared across the workspace —
  deterministic actor keypairs (`deterministic_keypair`, `ActorRegistry`),
  a pubkey alias table (`Aliases`) so printed transaction logs read in a
  test's own vocabulary instead of raw addresses, and lookup tables for
  Anchor error/instruction/event names. `TransactionHelpers` and
  `TransactionResult` grew alias-aware sends (`send_ok`, `send_err`,
  `send_err_named`) and log rendering (`logs_string`, `print_logs`) that
  read through the same table. Metaplex Token Metadata support
  (`MetaplexHelpers`, `MetadataArgs`, `Creator`, `TokenStandard`) and
  Token-2022 transfer-hook testing (`TransferHookTesting`) round out the
  framework-agnostic helpers.

- `anchor-litesvm`: `AnchorContext` gains the same alias-table integration
  (`ctx.alias`, `ctx.cast_actor`, `ctx.cast_mint`, `ctx.label`, `ctx.fund_ata`),
  context-owned send shortcuts (`send_ok`, `send_err`, `send_err_named`), a
  fluent build-and-send chain (`Tx`), Anchor event registration and decoding
  (`register_event`, `register_events_from_idl`, `EventHelpers`), and
  `BuildableIx` + `Program::build_ix`/`build_ix_with` — pairing an
  instruction's args type with a caller-supplied pubkey bundle so a test
  doesn't have to hand-fill an accounts struct field by field.

- `anchor-litesvm-derive` (new crate): `bundles_from_idl!`, a proc-macro invoked
  alongside `anchor_lang::declare_program!` against the same committed IDL.
  Per instruction, it emits a `<Ix>Bundle` struct (one `Pubkey` field per
  account the IDL can't infer), a `From<<Ix>Bundle> for <accounts struct>`
  that derives every PDA in dependency order and injects fixed addresses
  (the system program, well-known token programs), a `BuildableIx` pairing
  with the instruction's args type, and a `<account>_pda(...)` helper per
  derivable PDA. Adding an account to the program's IDL only requires
  regenerating; there's no hand-written builder to keep in sync. See
  `crates/anchor-litesvm/examples/basic_usage.rs` for a compiling example
  against a small vault program's IDL.

- An mdBook (`book/`) covering the reference and tutorial ground for the
  crate: a quickstart, the core concepts (the World, actor/alias setup,
  structured logs), three worked examples against real deployed programs
  (vault, escrow, stake), and a curated API reference. Every runtime code
  block is captured output from the crate's own tests, so the book can't
  drift from what the framework actually does. Build it with
  `mdbook build book`.

## [anchor-litesvm 0.4.0] - 2026-04-09

### Changed

- Upgraded `anchor-lang` from `1.0.0-rc.2` to `1.0.0`
- Upgraded `litesvm` from `0.8.2` to `0.11.0`
- Upgraded `litesvm-token` from `0.8.2` to `0.11.0`
- Updated the direct `litesvm-utils` dependency to `0.4.0`
- Migrated direct signer, hash, signature, transaction, and account usage from `solana-sdk` to split Solana crates:
  - `solana-account = 3.4.0`
  - `solana-hash = 3.1.0`
  - `solana-keypair = 3.1.2`
  - `solana-signature = 3.4.0`
  - `solana-signer = 3.0.0`
  - `solana-transaction = 3.0.1`
- Kept the remaining direct Solana dependency on the compatible Solana 3 line:
  - `solana-program = 3.0.0`

### Fixed

- Updated version-specific comments from Anchor `1.0.0-rc.2` to `1.0.0`
- Updated crate docs and examples to use split Solana imports and current version snippets
- Bundled crate-local examples so the published package no longer drops example entries during packaging
- Verified `cargo package -p anchor-litesvm --allow-dirty`
- Verified `cargo test -p anchor-litesvm --offline`

## [litesvm-utils 0.4.0] - 2026-04-09

### Changed

- Upgraded `litesvm` from `0.8.2` to `0.11.0`
- Upgraded `litesvm-token` from `0.8.2` to `0.11.0`
- Migrated direct signer and transaction usage from `solana-sdk` to split Solana crates:
  - `solana-keypair = 3.1.2`
  - `solana-signer = 3.0.0`
  - `solana-transaction = 3.0.1`
- Kept the remaining direct Solana dependencies on the current compatible Solana 3-line:
  - `solana-program = 3.0.0`
  - `solana-program-pack = 3.1.0`
  - `solana-system-interface = 2.0.0`

### Fixed

- Updated crate docs and README examples to use split Solana imports
- Verified `cargo package -p litesvm-utils --allow-dirty --offline`
- Verified `cargo test -p litesvm-utils --offline`

## [0.3.0] - 2025-01-12

### Breaking Changes

- **Rust 1.86+ required** - Updated dependencies require newer Rust version
- **anchor-lang** upgraded from 0.31.1 to 1.0.0-rc.2
- **litesvm** upgraded from 0.6.1 to 0.8.2
- **Solana SDK** upgraded from 2.2 to ~3.0
- **spl-token** upgraded from 7.0.0 to 9.0.0
- **spl-associated-token-account** upgraded from 6.0.0 to 8.0.0
- **thiserror** upgraded from 1.0 to 2.0

### Added

- Dedicated README.md for each crate (now displays correctly on crates.io)
- `solana-system-interface` dependency for system program instructions

### Changed

- `system_instruction` now imported from `solana_system_interface` instead of `solana_program`
- `add_program()` now returns `Result` and is handled with `.expect()`
- Simplified type conversions - anchor and litesvm now use same Solana SDK version
- Root README updated to be a workspace overview with links to individual crates

### Fixed

- Documentation now displays properly on crates.io for both crates

## [0.2.0] - Previous Release

Initial public release with:
- `anchor-litesvm`: Simplified Anchor testing with LiteSVM
- `litesvm-utils`: Framework-agnostic testing utilities
