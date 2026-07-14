# Aliases & Actors

Every scenario in this book has a cast, and the cast follows a crypto
convention: **Alice** and **Bob** are honest counterparties, **Charlie** an
honest third party, **Mallory** the attacker.

The point of using names instead of roles is that a test reads as a story:
"Mallory substitutes her own account" reads naturally, where
"attacker_keypair swaps accounts[3]" makes the reader do the translating work
themselves.

`cast_actor` casts a member of that cast:

```rust
let alice = ctx.cast_actor("Alice");
```

This does three things in one call: it derives a deterministic keypair from
the program id and the name `"Alice"` (same test, same program, same key
every run), airdrops it 100 SOL (far more than any scenario in this book
spends, so funding is never what fails a test), and registers
`alice.pubkey() -> "Alice"` in the context's alias table, the lookup this
book leans on throughout to print names instead of pubkeys.

Why deterministic, specifically? Because it matters for reproducing a
captured failure. Given the same program id, `"Alice"` is always the same
keypair, so reproducing a captured log needs no hardcoded pubkey pinned
anywhere; anyone can re-run the test and land on the exact same address
Alice had when the log was captured.

Every subsequent `send_ok` / `send_err` / `send_err_named` and `tx()` chain
draws on that alias table automatically.

That's the payoff: failures render in the test's own vocabulary. The vault
chapter's happy-path deposit runs as Alice, and the tree renders her name
directly:

```text
{{#include ../captured/vault_deposit.txt}}
```

Two things read off the alias table here. The `🔔 Deposited` badge resolves
`user` to `Alice` instead of a 44-character base58 key. The `Legend` at the
bottom lists every alias that appears in this render, mapped back to its real
address: `Alice` and the `vault` PDA.

The renderer only surfaces aliases it actually draws from: transaction
signers and frame program ids, not every account passed in an instruction's
metas. A mint or an ATA, or even a non-signing actor like escrow's maker,
won't show up in the legend unless it's also a signer or a program id.

> **Note:** well-known programs like `System` and `Token` are aliased by
> default too, but only non-default entries make the legend; a run touching
> only `System` prints no legend at all.

Casting isn't limited to signers, since not everything a scenario needs is
one. `cast_account` casts a passive, non-signing pubkey; `cast_mint` casts a
token mint; `fund_ata` funds a holder's associated token account and aliases
it `"<owner>/<mint>"`.

All of them register into the same alias table, so a token-heavy scenario
like the escrow example still reads by name end to end. See
[Setup](setup.md) for the full cast vocabulary.
