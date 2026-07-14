# Rebuild vendored program fixtures (.so + IDL) from examples/, then re-capture
# the book snapshots. Requires anchor + cargo-build-sbf.
.PHONY: fixtures book book-deps test drift

# vault/escrow pin anchor 1.0.2, which refuses to build on a keypair/declare_id
# mismatch (the vendored keypairs are throwaway, so --ignore-keys sidesteps it).
# staking pins 0.31.1 via its own Anchor.toml [toolchain] section; anchor's CLI
# auto-dispatches to that version with no avm state change, and 0.31.1 predates
# the mismatch check entirely (no flag needed, and --ignore-keys doesn't exist
# on it: passing it would error out).
fixtures:
	cd examples/vault   && anchor build --ignore-keys
	cd examples/escrow  && anchor build --ignore-keys
	cd examples/staking && anchor build
	cd examples/voting && anchor build --ignore-keys -- --features poll
	cp examples/voting/target/deploy/voting.so crates/anchor-litesvm/tests/fixtures/voting_poll.so
	cp examples/voting/target/idl/voting.json  crates/anchor-litesvm/idls/voting_poll.json
	cd examples/voting && anchor build --ignore-keys -- --features candidate
	cp examples/voting/target/deploy/voting.so crates/anchor-litesvm/tests/fixtures/voting_candidate.so
	cp examples/voting/target/idl/voting.json  crates/anchor-litesvm/idls/voting_candidate.json
	cd examples/voting && anchor build --ignore-keys -- --features vote
	cp examples/voting/target/deploy/voting.so crates/anchor-litesvm/tests/fixtures/voting_vote.so
	cp examples/voting/target/idl/voting.json  crates/anchor-litesvm/idls/voting_vote.json
	cd examples/voting && anchor build --ignore-keys -- --features guards
	cp examples/voting/target/deploy/voting.so crates/anchor-litesvm/tests/fixtures/voting_guarded.so
	cp examples/voting/target/idl/voting.json  crates/anchor-litesvm/idls/voting_guarded.json
	cp examples/vault/target/deploy/vault.so     crates/anchor-litesvm/tests/fixtures/vault.so
	cp examples/escrow/target/deploy/escrow.so   crates/anchor-litesvm/tests/fixtures/escrow.so
	cp examples/staking/target/deploy/staking.so crates/anchor-litesvm/tests/fixtures/staking.so
	cp examples/vault/target/idl/vault.json      crates/anchor-litesvm/idls/vault.json
	cp examples/escrow/target/idl/escrow.json    crates/anchor-litesvm/idls/escrow.json
	# staking's IDL embeds mpl-core's `Key`, which collides with anchor_lang::Key
	# under declare_program!'s glob; the sanitize pass namespaces it so the typed
	# client (tests/stake_typed.rs) can ingest it.
	cargo run -q -p anchor-litesvm --bin sanitize_idl -- examples/staking/target/idl/staking.json crates/anchor-litesvm/idls/staking.json
	BLESS=1 cargo test -p anchor-litesvm --tests

# Golden guard: fails if any committed snapshot drifts from current output.
drift test:
	cargo test -p anchor-litesvm --tests

# The book renders Mermaid diagrams via the mdbook-mermaid preprocessor, and
# uses mdBook 0.5+ native alert blockquotes (`> [!NOTE]`, `> [!WARNING]`,
# `> [!TIP]`) for callouts (those need no preprocessor). `make book-deps`
# installs the one preprocessor; its mermaid.min.js/mermaid-init.js assets are
# committed under book/, so a fresh clone builds once the binary is present.
book-deps:
	cargo install mdbook-mermaid

book:
	mdbook build book
