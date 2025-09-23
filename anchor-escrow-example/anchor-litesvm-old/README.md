# anchor-litesvm

[![Crates.io](https://img.shields.io/crates/v/anchor-litesvm.svg)](https://crates.io/crates/anchor-litesvm)
[![Documentation](https://docs.rs/anchor-litesvm/badge.svg)](https://docs.rs/anchor-litesvm)
[![License](https://img.shields.io/crates/l/anchor-litesvm.svg)](https://github.com/anchor-litesvm/anchor-litesvm#license)

A lightweight testing utility library that bridges Anchor and LiteSVM, dramatically simplifying the process of testing Anchor programs.

## Problem

Testing Anchor programs with LiteSVM currently requires significant boilerplate:
- Manual instruction discriminator calculation using SHA256
- Verbose instruction data serialization
- Complex account meta setup
- Manual account deserialization with proper type handling

## Solution

`anchor-litesvm` provides a minimal wrapper around LiteSVM that handles Anchor-specific patterns, reducing test code by 60-70% while maintaining full control and flexibility.

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
anchor-litesvm = "0.1.0"
```

## Quick Start

```rust
use anchor_litesvm::AnchorLiteSVM;

// Before: 5 lines of boilerplate
// After: 1 line!
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

// Everything else just works
let account = ctx.create_funded_account(10_000_000_000)?;
let mint = ctx.create_token_mint(&account, 9)?;
```

## Features

### 1. Fluent Instruction Builder with Direct Execution
Eliminates ~15 lines of boilerplate per instruction:

```rust
// Before (manual approach):
let mut hasher = Sha256::new();
hasher.update(b"global:make");
let hash = hasher.finalize();
let mut discriminator = [0u8; 8];
discriminator.copy_from_slice(&hash[..8]);

let mut instruction_data = discriminator.to_vec();
instruction_data.extend_from_slice(&seed.to_le_bytes());
instruction_data.extend_from_slice(&receive.to_le_bytes());
instruction_data.extend_from_slice(&amount.to_le_bytes());

let instruction = Instruction {
    program_id,
    accounts: vec![/* ... many lines of AccountMeta ... */],
    data: instruction_data,
};

// After (with anchor-litesvm):
// Option 1: Build instruction
let ix = ctx.build_instruction(
    "make",
    accounts,
    MakeArgs { seed, receive, amount }
);

// Option 2: Build and execute in one call!
let result = ctx.instruction_builder("make")
    .signer("maker", &maker)
    .account_mut("escrow", escrow_pda)
    .account("mint_a", mint_a)
    .system_program()
    .args(tuple_args((seed, receive, amount)))  // No struct needed!
    .execute(&mut ctx, &[&maker])?;
```

### 2. Type-Safe Account Deserialization
Automatic Anchor account unpacking:

```rust
// Before:
let account_data = svm.get_account(&escrow_pda).unwrap();
let escrow: EscrowState = EscrowState::try_from_slice(&account_data.data[8..]).unwrap();

// After:
let escrow: EscrowState = ctx.get_anchor_account(&escrow_pda)?;
```

### 3. Transaction Execution Helpers

Simplified transaction execution:

```rust
// Execute single instruction
let result = ctx.send_instruction(ix, &[&signer])?;
result.assert_success();

// Execute multiple instructions
let result = ctx.send_instructions(&[ix1, ix2], &[&signer])?;

// Build and execute in one call
let result = ctx.execute("transfer", accounts, args, &[&signer])?;

// Transaction result helpers
assert!(result.has_log("Transfer complete"));
println!("Used {} compute units", result.compute_units());
```

### 4. Test Account Helpers

Streamlined account creation for tests:

```rust
// Create funded account
let maker = ctx.create_funded_account(10_000_000_000)?;

// Create multiple accounts
let accounts = ctx.create_funded_accounts(5, 1_000_000_000)?;

// Create token mint
let mint = ctx.create_token_mint(&authority, 9)?;

// Create token account and mint tokens
let ata = ctx.create_token_account(
    &owner,
    &mint.pubkey(),
    Some((1_000_000_000, &mint_authority))  // Optional: mint tokens
)?;

// Batch airdrop
ctx.batch_airdrop(&[&account1, &account2], 1_000_000_000)?;
```

### 5. Assertion Helpers

Clean test assertions:

```rust
// Account assertions
ctx.assert_account_exists(&pda);
ctx.assert_account_closed(&old_account);
ctx.assert_accounts_closed(&[&escrow, &vault]);

// Token balance assertions
ctx.assert_token_balance(&ata, 1_000_000_000);
ctx.assert_token_balance_with_msg(&ata, expected, "Should have 1000 tokens");

// Lamports and owner assertions
ctx.assert_account_lamports(&account, 5_000_000_000);
ctx.assert_account_owner(&token_account, &spl_token::id());
```

### 6. Simplified Test Setup

Multiple ways to initialize your test environment:

```rust
// Simplest - one line setup
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

// Builder pattern for multiple programs
let mut ctx = AnchorLiteSVM::new()
    .deploy_program(program1_id, program1_bytes)
    .deploy_program(program2_id, program2_bytes)
    .build();

// Extension trait on Pubkey
use anchor_litesvm::ProgramTestExt;
let mut ctx = PROGRAM_ID.test_with(program_bytes);
```

### 7. Direct LiteSVM Access
The `AnchorContext` provides full access to the underlying LiteSVM instance:

```rust
let mut ctx = AnchorContext::new(svm, program_id);
// Direct access for any LiteSVM operations
ctx.svm.airdrop(&pubkey, amount);
ctx.svm.send_transaction(tx);
```

## Complete Example

```rust
use anchor_litesvm::{
    AnchorLiteSVM, TestHelpers, AssertionHelpers, tuple_args
};

#[test]
fn test_escrow_with_helpers() {
    // Setup - just one line!
    let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

    // Create accounts - one line each!
    let maker = ctx.create_funded_account(10_000_000_000)?;
    let mint_a = ctx.create_token_mint(&maker, 9)?;
    let maker_ata = ctx.create_token_account(
        &maker,
        &mint_a.pubkey(),
        Some((1_000_000_000, &maker))  // Mint 1000 tokens
    )?;

    // Execute instruction - fluent API with direct execution
    let result = ctx.instruction_builder("make")
        .signer("maker", &maker)
        .account_mut("escrow", escrow_pda)
        .account("mint_a", mint_a.pubkey())
        .system_program()
        .args(tuple_args((seed, amount)))  // No struct definition needed!
        .execute(&mut ctx, &[&maker])?;

    // Verify results
    result.assert_success();
    ctx.assert_account_exists(&escrow_pda);
    ctx.assert_token_balance(&vault, 1_000_000_000);
}
```

## Before/After Comparison

```rust
use anchor_litesvm::AnchorContext;
use litesvm::LiteSVM;

### Before: Raw LiteSVM (~50 lines for basic test)
```rust
// Create mint - multiple transactions
let mint = Keypair::new();
let rent = svm.minimum_balance_for_rent_exemption(82);
let create_mint_ix = system_instruction::create_account(...);
let init_mint_ix = spl_token::instruction::initialize_mint(...);
let tx = Transaction::new_signed_with_payer(&[create_mint_ix, init_mint_ix], ...);
svm.send_transaction(tx)?;

// Create ATA - another transaction
let create_ata_ix = create_associated_token_account(...);
let tx = Transaction::new_signed_with_payer(&[create_ata_ix], ...);
svm.send_transaction(tx)?;

// Mint tokens - yet another transaction
let mint_to_ix = mint_to(...);
let tx = Transaction::new_signed_with_payer(&[mint_to_ix], ...);
svm.send_transaction(tx)?;

// Execute instruction - manual everything
let discriminator = // 8 lines of SHA256 hashing
let accounts = vec![ // 10+ lines of AccountMeta
let ix = Instruction { ... };
let tx = Transaction::new_signed_with_payer(...);
let result = svm.send_transaction(tx)?;

// Manual assertions
let account = svm.get_account(&pda).unwrap();
assert!(account.lamports > 0);
let token_account = unpack(&account.data)?;
assert_eq!(token_account.amount, expected);
```

### After: With anchor-litesvm (~10 lines)
```rust
// Initialize with one line
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

// Create everything in one line each
let maker = ctx.create_funded_account(10_000_000_000)?;
let mint = ctx.create_token_mint(&maker, 9)?;
let ata = ctx.create_token_account(&maker, &mint.pubkey(), Some((1000, &maker)))?;

// Execute with fluent API
let result = ctx.instruction_builder("make")
    .signer("maker", &maker)
    .account_mut("escrow", escrow_pda)
    .args(tuple_args((seed, amount)))
    .execute(&mut ctx, &[&maker])?;

// Clean assertions
result.assert_success();
ctx.assert_token_balance(&vault, 1000);
```
```

## Roadmap

### Phase 1: Core Features (Complete)
- Automatic instruction discriminator calculation
- Instruction data serialization with Borsh
- Type-safe account deserialization
- PDA calculation helpers
- Direct LiteSVM access

### Phase 2: Enhanced Builder & Helpers (Complete)
- Fluent instruction builder API
- Tuple arguments support (no struct definition needed)
- Direct execution from builder (`.execute()`)
- Transaction execution helpers (`send_instruction`, `send_instructions`)
- Test account helpers (`create_funded_account`, `create_token_mint`, etc.)
- Assertion helpers for cleaner tests
- Transaction result wrapper with utilities

### Phase 3: Future Enhancements
- [ ] IDL file parsing for automatic account resolution
- [ ] Automatic signer detection from account types
- [ ] Event emission parsing from logs
- [ ] Time manipulation helpers
- [ ] Account snapshot/rollback for test isolation
- [ ] Procedural macros for test setup
- [ ] Integration with anchor-client types

## Design Principles

1. **Minimal API Surface** - Keep it simple and focused
2. **No Hidden Magic** - Direct access to LiteSVM, no functionality hidden
3. **Composable** - Works alongside litesvm-token and other utilities
4. **Type Safety** - Leverage Rust's type system for correctness
5. **Zero Overhead** - Thin wrapper, no performance penalties

## Comparison with Alternatives

| Feature | Raw LiteSVM | anchor-test | anchor-litesvm |
|---------|-------------|-------------|----------------|
| Setup Complexity | High | Medium | Low |
| Anchor Integration | Manual | Full | Targeted |
| Performance | Fastest | Slower | Fast |
| Flexibility | Full | Limited | Full |
| Lines of Code | ~50-200 | ~30-100 | ~10-30 |

## Key Benefits

- **70% Less Code**: Reduce test boilerplate by up to 70%
- **Readable Tests**: Fluent API makes tests self-documenting
- **Fast Iteration**: One-line account creation and instruction execution
- **Type Safety**: Full Rust type safety with Anchor integration
- **Zero Overhead**: Thin wrapper, no performance impact
- **Composable**: Works seamlessly with litesvm-token and other tools

## Contributing

Contributions are welcome! Priority areas:
- IDL parsing improvements
- Additional test utilities
- Documentation and examples
- Performance optimizations

## License

MIT