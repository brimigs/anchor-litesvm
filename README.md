# anchor-litesvm Workspace

Production-compatible testing framework for Solana programs using LiteSVM, with **78% less code** than raw LiteSVM.

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
Anchor-specific integration with **production-compatible syntax**:
- Native implementation of anchor-client API (no RPC overhead)
- Same syntax in tests and production code
- Automatic discriminator and serialization handling
- Direct integration with litesvm-utils helpers
- 40% faster compilation (no network dependencies)

```rust
use anchor_litesvm::AnchorLiteSVM;

// Production-compatible syntax without RPC!
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

// Exactly matches anchor-client syntax
let ix = ctx.program()
    .request()
    .accounts(my_program::client::accounts::Transfer { from, to })
    .args(my_program::client::args::Transfer { amount })
    .instructions()?[0];

ctx.execute_instruction(ix, &[&signer])?;
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

## Why anchor-litesvm Instead of anchor-client?

1. **No Network Dependencies** - 40% faster compilation, no reqwest/tokio/WebSocket deps
2. **No Mock RPC Setup** - One-line initialization vs complex mock client setup
3. **Integrated Test Helpers** - Token operations, assertions, and utilities built-in
4. **Production-Compatible Syntax** - Same API as anchor-client, transferable knowledge
5. **78% Less Code** - Dramatic reduction compared to raw LiteSVM

## Code Comparison

### Raw LiteSVM (493 lines)
```rust
// Manual everything
let discriminator = sha256::digest("global:make").as_bytes()[..8].to_vec();
let mut data = discriminator;
data.extend_from_slice(&seed.to_le_bytes());
// ... 20+ more lines for one instruction
```

### anchor-client + LiteSVM (279 lines)
```rust
// Requires mock RPC setup
let _mock_rpc = RpcClient::new_mock("succeeds".to_string());
let client = Client::new_with_options(Cluster::Custom(...), ...);
// ... still verbose token setup
```

### anchor-litesvm (106 lines - 78% reduction!)
```rust
// One-line setup, production syntax
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
let mint = ctx.svm.create_token_mint(&maker, 9)?;  // One-line token ops
let ix = ctx.program()  // Matches production exactly
    .request()
    .accounts(...)
    .args(...)
    .instructions()?[0];
```

## Key Features

### Production-Compatible Syntax ✅
The syntax is **identical** to anchor-client, making your test knowledge transferable:
```rust
// This exact code works in both tests AND production!
let ix = program
    .request()
    .accounts(my_program::client::accounts::Transfer { from, to })
    .args(my_program::client::args::Transfer { amount })
    .instructions()?[0];
```

### Integrated Test Helpers
- `create_token_mint()` - 1 line instead of 30+
- `mint_to()` - Direct minting without manual transactions
- `assert_token_balance()` - Clean, readable assertions
- `create_funded_account()` - Automatic SOL funding

### Performance Benefits
- 40% faster compilation (no network dependencies)
- No RPC client overhead
- Native LiteSVM integration
- Minimal dependency tree

## License

MIT