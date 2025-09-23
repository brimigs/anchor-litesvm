# anchor-litesvm Workspace

Testing utilities for Solana programs using LiteSVM, split into two specialized crates:

## Crate Structure

### `litesvm-utils`
General-purpose testing utilities for any Solana program:
- Framework agnostic - works with any Solana program
- Account creation and funding helpers
- Token operations (mints, accounts, minting)
- Transaction execution with result analysis
- Assertion helpers for testing
- PDA derivation utilities

```rust
use litesvm_utils::{LiteSVMBuilder, TestHelpers, AssertionHelpers};

// Works with any Solana program
let mut svm = LiteSVMBuilder::build_with_program(program_id, program_bytes);
let account = svm.create_funded_account(10_SOL)?;
let mint = svm.create_token_mint(&authority, 9)?;
```

### `anchor-litesvm`
Anchor-specific integration layer:
- Automatic discriminator calculation for Anchor instructions
- Anchor account deserialization with proper type handling
- Fluent instruction builder with tuple arguments
- Path toward anchor-client compatibility (WIP)

```rust
use anchor_litesvm::{AnchorLiteSVM, tuple_args};

// Anchor-specific conveniences
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
let result = ctx.instruction_builder("transfer")
    .signer("from", &maker)
    .account_mut("to", recipient)
    .args(tuple_args((amount,)))  // No struct needed!
    .execute(&mut ctx, &[&maker])?;
```

## Installation

Add to your `Cargo.toml`:

```toml
# For general Solana testing
[dev-dependencies]
litesvm-utils = "0.1"

# For Anchor programs (includes litesvm-utils)
[dev-dependencies]
anchor-litesvm = "0.1"
```

## Why Two Crates?

1. **Clean separation of concerns** - General utilities vs Anchor-specific code
2. **Reusability** - Non-Anchor projects can use just `litesvm-utils`
3. **Smaller dependencies** - Only pull in what you need
4. **Future compatibility** - `anchor-litesvm` will integrate with `anchor-client` for consistent production/testing syntax

## Migration from v0.0.x

The original `anchor-litesvm` has been split:
- General helpers → `litesvm-utils`
- Anchor-specific code → `anchor-litesvm`

Most code will continue to work as `anchor-litesvm` re-exports `litesvm-utils` functionality.

## Examples

See the `examples/` directory for usage patterns:
- `workspace_demo.rs` - Overview of both crates
- `basic_usage.rs` - Simple test setup
- `advanced_features.rs` - Complex testing scenarios

## Future Development

### Coming Soon
- Full `anchor-client` integration for identical syntax between testing and production
- IDL-based instruction building with automatic account resolution
- Enhanced error messages with program log parsing

### Goal
Make testing syntax identical to production:
```rust
// Same code for testing AND production!
let program = client.program(program_id);
program.request()
    .accounts(my_program::accounts::Transfer { from, to })
    .args(my_program::instruction::Transfer { amount })
    .send()?;
```

## License

MIT