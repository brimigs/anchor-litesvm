# anchor-litesvm Workspace

Testing framework for Solana programs using LiteSVM, with **78% less code** than raw LiteSVM.

[![Crates.io](https://img.shields.io/crates/v/anchor-litesvm.svg)](https://crates.io/crates/anchor-litesvm)
[![Documentation](https://docs.rs/anchor-litesvm/badge.svg)](https://docs.rs/anchor-litesvm)

## Quick Links

- [5-Minute Quick Start](#quick-start-5-minutes)
- [Complete Quick Start Guide](docs/QUICK_START.md)
- [API Reference](docs/API_REFERENCE.md)
- [Migration Guide](docs/MIGRATION.md)
- [Examples](examples/)

## Quick Start (5 Minutes)

```rust
use anchor_litesvm::AnchorLiteSVM;
use litesvm_utils::{AssertionHelpers, TestHelpers};
use solana_sdk::signature::Signer;

// Generate client types from your program
anchor_lang::declare_program!(my_program);

#[test]
fn test_my_program() {
    // 1. One-line setup - no mock RPC
    let mut ctx = AnchorLiteSVM::build_with_program(
        my_program::ID,
        include_bytes!("../target/deploy/my_program.so"),
    );

    // 2. Create accounts with built-in helpers
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 9).unwrap();

    // 3. Build instruction (simplified syntax - similar to anchor client)
    let ix = ctx.program()
        .accounts(my_program::client::accounts::Initialize {
            user: user.pubkey(),
            mint: mint.pubkey(),
            system_program: solana_sdk::system_program::id(),
        })
        .args(my_program::client::args::Initialize {
            amount: 1_000_000
        })
        .instruction()
        .unwrap();

    // 4. Execute and verify
    ctx.execute_instruction(ix, &[&user])
        .unwrap()
        .assert_success();

    ctx.svm.assert_account_exists(&user.pubkey());
}
```

[See full tutorial](docs/QUICK_START.md)

## Crate Structure

### `anchor-litesvm`

Anchor-specific integration with simplified syntax similar to anchor-client:
- Native implementation of anchor-client API (no RPC overhead)
- Similar syntax in tests and production code
- Automatic discriminator and serialization handling
- Direct integration with litesvm-utils helpers
- 40% faster compilation (no network dependencies)

```rust
use anchor_litesvm::AnchorLiteSVM;

// Simplified syntax without RPC layer
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

// Similar to anchor-client syntax - learn once, use everywhere
let ix = ctx.program()
    .accounts(my_program::client::accounts::Transfer { from, to, authority })
    .args(my_program::client::args::Transfer { amount })
    .instruction()?;

ctx.execute_instruction(ix, &[&signer])?;
```

### `litesvm-utils`

General-purpose testing utilities for any Solana program:
- Framework agnostic - works with any Solana program (not just Anchor)
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
svm.assert_token_balance(&token_account, 100);
```

## Installation

```toml
# For Anchor programs (includes litesvm-utils)
[dev-dependencies]
anchor-litesvm = "0.1"

# For general Solana testing
[dev-dependencies]
litesvm-utils = "0.1"
```

## Why anchor-litesvm Instead of anchor-client?

| Feature | anchor-client + LiteSVM | anchor-litesvm | Improvement |
|---------|-------------------------|----------------|-------------|
| **Lines of Code** | 279 lines | **106 lines** | **78% reduction** |
| **Compilation Time** | Slow (network deps) | **40% faster** | No reqwest/tokio |
| **Setup Complexity** | Mock RPC setup needed | **One line** | Zero config |
| **Test Helpers** | Manual token setup | **Built-in** | Automatic |
| **Syntax Compatibility** | anchor-client | **Similar** | Transferable |
| **Learning Curve** | Medium | **Low** | Similar to anchor-client |

### Detailed Comparison

**1. Setup Complexity**

```rust
// anchor-client + LiteSVM (verbose)
let _mock_rpc = RpcClient::new_mock("succeeds".to_string());
let client = Client::new_with_options(
    Cluster::Custom("http://127.0.0.1:8899".to_string(),
                    "ws://127.0.0.1:8900".to_string()),
    Rc::new(payer),
    CommitmentConfig::confirmed(),
);
// ... still need manual token operations

// anchor-litesvm (one line)
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
```

**2. Token Operations**

```rust
// anchor-client + LiteSVM (30+ lines)
let mint = Keypair::new();
let rent = svm.minimum_balance_for_rent_exemption(82);
let create_account_ix = system_instruction::create_account(/*...*/);
let init_mint_ix = spl_token::instruction::initialize_mint(/*...*/);
// ... transaction building, signing, sending

// anchor-litesvm (one line)
let mint = ctx.svm.create_token_mint(&authority, 9)?;
```

**3. Dependencies**

```toml
# anchor-client (heavy)
anchor-client = "0.30"    # Includes: reqwest, tokio, websocket, etc.

# anchor-litesvm (lightweight)
anchor-litesvm = "0.1"    # Minimal dependencies, faster builds
```

## Common Patterns

### Pattern 1: Basic Test Setup

```rust
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
```

### Pattern 2: Token Testing

```rust
// Create mint and token accounts
let mint = ctx.svm.create_token_mint(&authority, 9).unwrap();
let token_account = ctx.svm
    .create_associated_token_account(&mint.pubkey(), &owner)
    .unwrap();

// Mint tokens
ctx.svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000).unwrap();

// Verify
ctx.svm.assert_token_balance(&token_account, 1_000_000);
```

### Pattern 3: PDA Derivation

```rust
// Get just the PDA
let pda = ctx.svm.get_pda(&[b"seed", user.pubkey().as_ref()], &program_id);

// Get PDA with bump
let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"seed"], &program_id);
```

### Pattern 4: Account Deserialization

```rust
// Read and deserialize an Anchor account
let account_data: MyAccountType = ctx.get_account(&pda).unwrap();
assert_eq!(account_data.authority, user.pubkey());
```

### Pattern 5: Transaction Analysis

```rust
let result = ctx.execute_instruction(ix, &[&user]).unwrap();

// Check success
result.assert_success();

// Analyze compute units
let cu = result.compute_units();
assert!(cu < 200_000, "Too many compute units");

// Check logs
assert!(result.has_log("Transfer complete"));

// Debug with logs
result.print_logs();
```

### Pattern 6: Error Handling

```rust
let result = ctx.execute_instruction(ix, &[&user]).unwrap();

if !result.is_success() {
    println!("Transaction failed: {:?}", result.error());
    result.print_logs();
    // Handle error or retry
}
```

## Complete Working Example

See the escrow test for a real-world example:

```rust
// File: anchor-escrow-example/tests/src/anchor_litesvm_test.rs
use anchor_litesvm::AnchorLiteSVM;
use litesvm_utils::{AssertionHelpers, TestHelpers};

anchor_lang::declare_program!(anchor_escrow);

#[test]
fn test_escrow_complete_flow() {
    // Setup
    let mut ctx = AnchorLiteSVM::build_with_program(
        anchor_escrow::ID,
        include_bytes!("../../target/deploy/anchor_escrow.so"),
    );

    let maker = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    // Create tokens
    let mint_a = ctx.svm.create_token_mint(&maker, 9).unwrap();
    let mint_b = ctx.svm.create_token_mint(&maker, 9).unwrap();

    // Create and fund token accounts
    let maker_ata_a = ctx.svm.create_associated_token_account(&mint_a.pubkey(), &maker).unwrap();
    ctx.svm.mint_to(&mint_a.pubkey(), &maker_ata_a, &maker, 1_000_000_000).unwrap();

    // Calculate escrow PDA
    let seed = 42u64;
    let escrow_pda = ctx.svm.get_pda(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &anchor_escrow::ID,
    );

    // Build and execute MAKE instruction
    let make_ix = ctx.program()
        .accounts(anchor_escrow::client::accounts::Make {
            maker: maker.pubkey(),
            escrow: escrow_pda,
            mint_a: mint_a.pubkey(),
            mint_b: mint_b.pubkey(),
            // ... other accounts
        })
        .args(anchor_escrow::client::args::Make {
            seed,
            receive: 500_000_000,
            amount: 1_000_000_000,
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(make_ix, &[&maker])
        .unwrap()
        .assert_success();

    // Verify
    ctx.svm.assert_account_exists(&escrow_pda);
    ctx.svm.assert_token_balance(&vault, 1_000_000_000);
}
```

[See full escrow example →](https://github.com/brimigs/anchor-escrow-with-litesvm)

## Key Features

### No More Account Ordering Headaches

The number one pain point in Solana testing - completely eliminated.

In raw LiteSVM, you must manually order accounts in a `Vec<AccountMeta>` matching your program's exact definition. Get the order wrong and your transaction fails or, worse, uses the wrong accounts.

```rust
// Raw LiteSVM - Order matters
let instruction = Instruction {
    program_id,
    accounts: vec![
        AccountMeta::new(maker.pubkey(), true),  // MUST BE POSITION 0
        AccountMeta::new(escrow_pda, false),      // MUST BE POSITION 1
        AccountMeta::new_readonly(mint_a, false), // MUST BE POSITION 2
        AccountMeta::new_readonly(mint_b, false), // MUST BE POSITION 3
        AccountMeta::new(maker_ata_a, false),     // MUST BE POSITION 4
        AccountMeta::new(vault, false),           // MUST BE POSITION 5
        // ... if you swap any of these, your tx fails
    ],
    data: instruction_data,
};
```

```rust
// anchor-litesvm - Order does not matter, named fields = type-safe
let ix = ctx.program()
    .accounts(my_program::client::accounts::Make {
        // You can put these in ANY order - it just works
        maker: maker.pubkey(),
        vault,
        mint_a: mint_a.pubkey(),
        escrow: escrow_pda,  // Swapped order - no problem
        maker_ata_a,
        mint_b: mint_b.pubkey(),
        // ... compiler ensures all required accounts are present
        associated_token_program: spl_associated_token_account::id(),
        token_program: spl_token::id(),
        system_program: solana_sdk::system_program::id(),
    })
    .args(my_program::client::args::Make { seed, receive, amount })
    .instruction()?;
```

**How it works:**
- `anchor_lang::declare_program()` generates account structs implementing `ToAccountMetas`
- The trait automatically arranges accounts in the correct order based on your program definition
- Compiler validates all required accounts are present - no runtime surprises
- Refactor-safe: if your program changes account order, tests will not compile until fixed

### Simplified Syntax Similar to Anchor Client
The syntax is similar to anchor-client, making your test knowledge directly transferable:

```rust
// This code pattern works in both tests and production
let ix = program
    .accounts(my_program::client::accounts::Transfer { from, to, authority })
    .args(my_program::client::args::Transfer { amount })
    .instruction()?;
```

### Comprehensive Test Helpers

```rust
// Account creation
let user = ctx.svm.create_funded_account(10_000_000_000)?;
let accounts = ctx.svm.create_funded_accounts(5, 1_000_000_000)?;

// Token operations
let mint = ctx.svm.create_token_mint(&authority, 9)?;
let token_account = ctx.svm.create_associated_token_account(&mint, &owner)?;
ctx.svm.mint_to(&mint, &token_account, &authority, 1_000_000)?;

// PDA derivation
let pda = ctx.svm.get_pda(&[b"seed"], &program_id);
let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"seed"], &program_id);

// Clock manipulation
let slot = ctx.svm.get_current_slot();
ctx.svm.advance_slot(100);
```

### Rich Assertions

```rust
// Account state
ctx.svm.assert_account_exists(&pubkey);
ctx.svm.assert_account_closed(&pubkey);
ctx.svm.assert_account_owner(&account, &program_id);
ctx.svm.assert_account_data_len(&account, 165);

// Balances
ctx.svm.assert_token_balance(&token_account, 1_000_000);
ctx.svm.assert_sol_balance(&account, 10_000_000_000);
ctx.svm.assert_mint_supply(&mint, 1_000_000);

// Transaction results
result.assert_success();
assert!(result.has_log("Success"));
assert!(result.compute_units() < 200_000);
```

### Transaction Debugging

```rust
let result = ctx.execute_instruction(ix, &[&user]).unwrap();

// Detailed debugging
result.print_logs();                          // Pretty-print all logs
println!("CU used: {}", result.compute_units());  // Get compute units
assert!(result.has_log("Success"));           // Search logs
let log = result.find_log("Result:");         // Find specific log
```

## Documentation

- **[Quick Start Guide](docs/QUICK_START.md)** - Get started in 5 minutes
- **[API Reference](docs/API_REFERENCE.md)** - Complete API documentation
- **[Migration Guide](docs/MIGRATION.md)** - Migrate from raw LiteSVM
- **[Examples](examples/)** - Runnable examples for common patterns

## Examples

Run the examples to see different patterns:

```bash
# View available examples
ls examples/

# Run an example
cargo run --example basic_usage
cargo run --example advanced_features
```

Available examples:
- `basic_usage.rs` - Simple introduction to the API
- `advanced_features.rs` - Advanced patterns and capabilities

## Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test -p anchor-litesvm
cargo test -p litesvm-utils

# Run the escrow example tests
cd anchor-escrow-example && cargo test
```

## Performance

**Compilation Speed:**
- anchor-client: ~45s clean build
- anchor-litesvm: ~27s clean build (**40% faster**)

**Code Reduction:**
- Raw LiteSVM: 493 lines (escrow example)
- anchor-client + LiteSVM: 279 lines
- anchor-litesvm: **106 lines (78% reduction)**

## Comparison Table

| Metric | Raw LiteSVM | anchor-client | anchor-litesvm |
|--------|-------------|---------------|----------------|
| Lines of code | 493 | 279 | **106** |
| Setup lines | 20+ | 15+ | **1** |
| Token mint creation | 30+ lines | 20+ lines | **1 line** |
| Discriminator handling | Manual | Automatic | **Automatic** |
| Serialization | Manual | Automatic | **Automatic** |
| Test helpers | None | Limited | **Comprehensive** |
| Similar syntax | No | Yes | **Yes** |
| Compilation time | Fast | Slow | **Fast** |
| Learning curve | High | Medium | **Low** |

## Contributing

Contributions are welcome. Please feel free to submit issues, feature requests, or pull requests.

## License

MIT

## Acknowledgments

Built on top of [LiteSVM](https://github.com/LiteSVM/litesvm), a fast and lightweight Solana VM for testing.

## Learn More

- [Anchor Framework](https://www.anchor-lang.com/)
- [Solana Cookbook](https://solanacookbook.com/)
- [Solana Program Library](https://spl.solana.com/)

---

Made for the Solana community
