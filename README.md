# anchor-litesvm Workspace

**Two powerful crates for Solana program testing with LiteSVM**

| Crate | Description | crates.io | docs.rs |
|-------|-------------|-----------|---------|
| **[anchor-litesvm](crates/anchor-litesvm)** | Anchor-specific testing with simplified syntax | [![Crates.io](https://img.shields.io/crates/v/anchor-litesvm.svg)](https://crates.io/crates/anchor-litesvm) | [![docs.rs](https://docs.rs/anchor-litesvm/badge.svg)](https://docs.rs/anchor-litesvm) |
| **[litesvm-utils](crates/litesvm-utils)** | Framework-agnostic testing utilities | [![Crates.io](https://img.shields.io/crates/v/litesvm-utils.svg)](https://crates.io/crates/litesvm-utils) | [![docs.rs](https://docs.rs/litesvm-utils/badge.svg)](https://docs.rs/litesvm-utils) |

## Which Crate Should I Use?

### Use `anchor-litesvm` if:
- You're testing **Anchor programs**
- You want simplified syntax similar to anchor-client
- You need Anchor account deserialization and event parsing

### Use `litesvm-utils` if:
- You're testing **Native Solana**, **SPL**, or **non-Anchor** programs
- You want framework-agnostic utilities
- You're building your own testing framework

> **Note:** `anchor-litesvm` includes all of `litesvm-utils`, so Anchor users get everything automatically.

## Crate Relationship

```
┌─────────────────────────────────────┐
│         anchor-litesvm              │
│  (Anchor-specific features)         │
│  • Simplified syntax                │
│  • Account deserialization          │
│  • Event parsing                    │
│  • Discriminator handling           │
└─────────────┬───────────────────────┘
              │ builds upon
              ▼
┌─────────────────────────────────────┐
│         litesvm-utils               │
│  (Framework-agnostic utilities)     │
│  • Account creation & funding       │
│  • Token operations                 │
│  • Transaction helpers              │
│  • Assertions                       │
│  • PDA derivation                   │
└─────────────┬───────────────────────┘
              │ uses
              ▼
┌─────────────────────────────────────┐
│           LiteSVM                   │
│  (Fast Solana VM for testing)       │
└─────────────────────────────────────┘
```

## Quick Start

### For Anchor Programs

```toml
[dev-dependencies]
anchor-litesvm = "0.4"
```

```rust
use anchor_litesvm::AnchorLiteSVM;
use litesvm_utils::{AssertionHelpers, TestHelpers};

// Generate `my_program::client::{accounts, args}` from the IDL, and pair
// them with a caller-facing pubkey bundle per instruction from that IDL.
anchor_lang::declare_program!(my_program);
anchor_litesvm::bundles_from_idl!(my_program);

#[test]
fn test_my_program() {
    // One-line setup. The name registers as a pubkey alias, so a failing
    // send's printed logs read `my_program` instead of the raw program ID.
    let mut ctx = AnchorLiteSVM::build_with_program(
        my_program::ID,
        "my_program",
        include_bytes!("../target/deploy/my_program.so"),
    );

    // Create accounts
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    // Build the instruction from a bundle: only the accounts the IDL can't
    // infer are fields; PDAs and fixed addresses are derived/injected.
    let ix = ctx.program().build_ix(
        InitializeBundle { user: user.pubkey() },
        my_program::client::args::Initialize { amount: 100 },
    );

    // Execute and verify
    ctx.execute_instruction(ix, &[&user]).unwrap().assert_success();
}
```

See [`crates/anchor-litesvm/examples/basic_usage.rs`](crates/anchor-litesvm/examples/basic_usage.rs)
for a fully compiling version of this flow.

### For Non-Anchor Programs

```toml
[dev-dependencies]
litesvm-utils = "0.4"
```

```rust
use litesvm_utils::{LiteSVMBuilder, TestHelpers, AssertionHelpers, TransactionHelpers};

#[test]
fn test_my_program() {
    // Setup
    let mut svm = LiteSVMBuilder::build_with_program(program_id, &program_bytes);

    // Create accounts and tokens
    let user = svm.create_funded_account(10_000_000_000).unwrap();
    let mint = svm.create_token_mint(&user, 9).unwrap();

    // Execute and verify
    let result = svm.send_instruction(ix, &[&user]).unwrap();
    result.assert_success();
    svm.assert_token_balance(&token_account, 1_000_000);
}
```

## Why These Crates?

| Metric | Raw LiteSVM | anchor-client | anchor-litesvm |
|--------|-------------|---------------|----------------|
| Lines of code | 493 | 279 | **106** |
| Setup lines | 20+ | 15+ | **1** |
| Token mint creation | 30+ lines | 20+ lines | **1 line** |
| Compilation | Fast | Slow | **Fast** |
| Mock RPC needed | No | Yes | **No** |

## Documentation

- **[The book](book/)** - Quickstart, concepts, and worked examples (vault,
  escrow, stake) against real deployed programs, plus a curated API
  reference; build it locally with `mdbook build book`
- **[anchor-litesvm README](crates/anchor-litesvm/README.md)** - Anchor-specific features
- **[litesvm-utils README](crates/litesvm-utils/README.md)** - Framework-agnostic utilities
- **[basic_usage example](crates/anchor-litesvm/examples/basic_usage.rs)** - Runnable end-to-end walkthrough

The longer-form guides below predate the IDL-driven bundle API and are awaiting a refresh:

- **[Quick Start Guide](docs/QUICK_START.md)** - 5-minute tutorial (refresh pending)
- **[API Reference](docs/API_REFERENCE.md)** - Complete API docs (refresh pending)
- **[Migration Guide](docs/MIGRATION.md)** - Migrate from raw LiteSVM (refresh pending)

## Examples

```bash
# Run examples
cargo run -p anchor-litesvm --example basic_usage
cargo run -p anchor-litesvm --example advanced_features
```

## Testing

```bash
# Run all tests (67 total)
cargo test

# Run specific crate tests
cargo test -p anchor-litesvm    # 11 tests
cargo test -p litesvm-utils     # 56 tests
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built on top of [LiteSVM](https://github.com/LiteSVM/litesvm), a fast and lightweight Solana VM for testing.
