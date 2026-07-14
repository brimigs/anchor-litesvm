# Introduction

`anchor-litesvm` drives Anchor programs through [LiteSVM](https://github.com/LiteSVM/litesvm),
a fast in-process Solana VM. On top of it sits an ergonomic testing surface,
built on three ideas worth naming before you meet them in code.

Your IDL (Interface Definition Language: Anchor's generated JSON description of
a program's instructions, accounts, and events) drives a typed instruction
builder, so a test calls ordinary Rust functions with named fields instead of
hand-packing instruction bytes.

A named cast of actors: funded, aliased keypairs standing in for the people
driving the test, so a scenario reads "Alice deposits" instead of one raw
pubkey doing something to another.

And structured transaction logs, rendered as a CPI tree: a nested view of
every cross-program invocation a transaction made, one line per call, so a
failure or an emitted event shows up in context instead of buried in a wall of
base64.

This book is a tutorial and reference. Every block of program output you see is
**captured from a real test** in this repository (a deployed `.so`, a real
transaction), not hand-typed, so it cannot drift from what the code emits.

## The two crates

- **`anchor-litesvm`**: Anchor-specific testing. Generated instruction bundles,
  account/event decoding, the `AnchorContext` world.
- **`litesvm-utils`**: the framework-agnostic helpers underneath it (actors, aliases,
  token setup, clock warping, the log renderer). `anchor-litesvm` re-exports it,
  so Anchor users get everything.

## What you will build

Three worked examples, each a real deployed program:

- **Vault** (`initialize`/`deposit`/`withdraw`/`close`, emits a `Deposited` event):
  the simplest happy path, event decoding, and the negative-account escape hatch.
- **Escrow** (`make`/`take`/`refund`, SPL token CPIs, a 90-day time-lock): token
  setup, the CPI tree, and time-travel via the clock helpers.
- **Stake** (mpl-core NFT staking, a freeze-period day lock): the deepest CPI tree,
  and driving a program with raw hand-built instructions.
