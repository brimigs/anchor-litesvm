# Structured Logs

Every `send_ok` / `send_err` / `send_err_named` call returns a
`TransactionResult`. Its `tree_string()` method renders the run's raw logs
as a CPI tree: one line per invoke frame, decoded events as badges, a
failing leaf named instead of a bare error code, and a legend mapping every
alias back to its address.

This is the anatomy the rest of the book's captures are made of. This
chapter walks it line by line against the vault deposit capture, then the
negative-path capture for the failure-specific parts.

## Anatomy of a passing run

```text
{{#include ../captured/vault_deposit.txt}}
```

- `── vault::Deposit ──` is the title bar: the top frame's program (alias
  `vault`) and, when the logs name it, the instruction (`Deposit`).
- `Transaction  signers=[Alice]` lists the transaction's required-signature
  keys, alias-resolved. One line, regardless of how many frames follow.
- `└── vault::Deposit [1] ✓ 6874cu  signer=Alice` is the top-level frame:
  `[1]` is the invoke depth, `✓` the outcome, `6874cu` the compute units
  *this frame* consumed, and `signer=Alice` names who signed for it (only
  top-level frames carry a signer annotation).
- `├── System [2] ✓ (no cu)` is a nested CPI one level deeper (`[2]`): the
  lamport transfer `deposit` makes into the vault PDA via `system_program`.
  `(no cu)` appears when the runtime's logs don't report a per-frame figure
  for that invocation, not when the frame is somehow free.
- `└── 🔔 Deposited { user: Alice, amount: 1000000000, vault_balance:
  1000000000 }` is a decoded event badge: a leaf sibling of the CPI frames,
  inside the frame that emitted it. `register_events_from_idl` (see
  [Setup](setup.md)) is what makes this renderable at all; without a
  registered decoder for `Deposited`, the raw base64 payload would print
  instead. `user` reads `Alice` for the same reason everything else does:
  the decoder resolves pubkey fields through the alias table.
- The footer reports total compute units and the fee charged, then
  `Legend (2):` lists the two non-default aliases this run actually
  touched, `Alice` and `vault`, next to their real addresses. See
  [Aliases & Actors](aliases.md) for why `System` doesn't appear here too.

`├──`/`└──` connectors and indentation track invoke depth and whether a
frame is the last child at its level: standard tree-drawing rules. A frame
with siblings after it gets `├──`; the last sibling gets `└──`.

## Anatomy of a failing run

The `✗` mark and a leaf under the failing frame name the failure directly,
so you never have to decode a raw error number by hand to see what broke:

```text
{{#include ../captured/vault_wrong_state.txt}}
```

`└── vault::Deposit [1] ✗ 5225cu  signer=Alice` then `└── Error:
ConstraintSeeds` names the constraint that rejected the swapped account,
resolved from the `Error Code: ConstraintSeeds. Error Number: 2006.` line
Anchor itself logs (an `AnchorError`).

The `Error: InstructionError(0, Custom(2006))` line beneath the tree is the
raw `TransactionError` the runtime returned. It's always there on failure;
the named leaf above it is what makes the tree readable without decoding
the custom code by hand.

Not every failure comes from an Anchor-logged error, though: a program
built without an IDL has no `Error Code:` log line to read, so its custom
codes need a name registered by hand with `register_program_errors`
(`FreezePeriodNotElapsed` in the [Stake](../examples/stake.md) chapter).

Because both sources can be in play, the failure leaf resolves in a fixed
order: the `AnchorError` log line first, then the registered error-name
table, then the raw error as a last resort. Escrow's and Vault's failure
leaves (`ConstraintSeeds`, `ConstraintTokenOwner`, `EscrowExpired`, ...) all
come from Anchor's own logs; see the [Vault](../examples/vault.md) and
[Escrow](../examples/escrow.md) chapters for those captures.

## Sending: `send_ok` / `send_err` / `send_err_named`

The three context-level senders share a signature and differ only in what
they assert:

- `ctx.send_ok(ix, &[&signer])` asserts the transaction succeeds.
- `ctx.send_err(ix, &[&signer])` asserts it fails, any error.
- `ctx.send_err_named(ix, &[&signer], "ConstraintSeeds")` asserts it fails
  *and* that the failure resolves to (or its logs contain) the given name.

All three return the `TransactionResult` whose `tree_string()` is what's
captured above.

The fluent `ctx.tx(&[&signer]).build(bundle, args)` chain terminates in the
same three verbs (`.send_ok()`, `.send_err()`, `.send_err_named("Name")`),
so whichever style a test uses, the result is the same kind of value with
the same rendering.

## Printing vs. capturing

`result.print_logs()` prints the tree to stdout and returns `self`, so it
chains at the end of a call (`ctx.send_ok(ix, signers).print_logs();`).

`result.tree_string()` returns the same content as a `String` instead of
printing it. That's the method this book actually relies on: every
`{{#include}}` block in this book is a `tree_string()` capture, verbatim,
checked against a committed fixture, so it can't drift from what the code
actually renders.
