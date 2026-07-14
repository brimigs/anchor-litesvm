# The World

`AnchorContext` (conventionally bound as `ctx`) is the World: a stage you set
once and direct through the rest of the test.

It owns the `LiteSVM` instance (`ctx.svm`), the alias table you just met in
[Aliases & Actors](aliases.md), the cast of actors it has minted, and the
event registry that decodes a program's `emit!`ed logs. Everything else in
this book is a method on it, or a value it hands back.

Why route everything through one object instead of passing an `svm` handle
and an alias map around separately? Because it lets a scenario get described
in terms of actors, not raw pubkeys and byte layouts: "Alice deposits",
"Mallory substitutes her own account", rather than "airdrop this key, derive
that PDA, splice account index 3".

[Setup](setup.md) covers the methods that do the describing.

You might expect this to be a testing convenience layered on top of how
Solana actually works, a friendly fiction. It isn't. Everything on Solana is
an account: an actor is a keypair, which owns an account; a PDA, a mint, a
token account are all accounts too, just non-signing ones.

Casting every participant in a scenario, signer or not, as a named entry in
one alias table isn't an abstraction bolted onto the account model: it's the
same model, given names.

The cast doesn't just act on the World; they observe it. A `send_ok` /
`send_err` / `send_err_named` call returns a `TransactionResult`, the record
of what happened, in the World's own vocabulary: which frames ran, which
account was whose, what a decoded event said, what failed and why.

That record is what [Structured Logs](structured-logs.md) covers:
`tree_string()` renders it as a CPI tree, aliases resolved, so reading a
failure back is reading what the actors themselves would have witnessed on
the stage, not a raw byte dump.
