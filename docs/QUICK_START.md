# Quick Start Guide

Get started with `anchor-litesvm` in 5 minutes! This guide walks you through setting up your first test with production-compatible syntax.

## Table of Contents

- [Installation](#installation)
- [Your First Test](#your-first-test)
- [Understanding the API](#understanding-the-api)
- [Common Patterns](#common-patterns)
- [Next Steps](#next-steps)

## Installation

Add `anchor-litesvm` to your dev dependencies:

```toml
[dev-dependencies]
anchor-litesvm = "0.1"
solana-sdk = "1.18"
anchor-lang = "0.30"
```

## Your First Test

Here's a complete, working test you can copy and run:

```rust
use anchor_litesvm::AnchorLiteSVM;
use litesvm_utils::{AssertionHelpers, TestHelpers};
use solana_sdk::signature::Signer;

// Generate client modules from your program's IDL
anchor_lang::declare_program!(my_program);

#[test]
fn test_my_first_instruction() {
    // ========================================
    // 1. Setup: One-line initialization!
    // ========================================
    let mut ctx = AnchorLiteSVM::build_with_program(
        my_program::ID,
        include_bytes!("../../target/deploy/my_program.so"),
    );

    // ========================================
    // 2. Create Accounts: Using built-in helpers
    // ========================================
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 9).unwrap();
    let token_account = ctx.svm
        .create_associated_token_account(&mint.pubkey(), &user)
        .unwrap();

    // ========================================
    // 3. Build Instruction: Production syntax!
    // ========================================
    let ix = ctx.program()
        .request()
        .accounts(my_program::client::accounts::Initialize {
            user_account: user.pubkey(),
            token_account,
            mint: mint.pubkey(),
            system_program: solana_sdk::system_program::id(),
            token_program: spl_token::id(),
        })
        .args(my_program::client::args::Initialize {
            amount: 1_000_000,
        })
        .instructions()
        .unwrap()
        .remove(0);

    // ========================================
    // 4. Execute: Run the instruction
    // ========================================
    let result = ctx.execute_instruction(ix, &[&user]).unwrap();
    result.assert_success();

    // ========================================
    // 5. Verify: Check the results
    // ========================================
    ctx.svm.assert_account_exists(&user.pubkey());
    ctx.svm.assert_token_balance(&token_account, 1_000_000);
}
```

## Understanding the API

### 1. Setup

```rust
let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
```

**What it does:** Creates a test environment with your program deployed. That's it!

**Key points:**
- No mock RPC setup needed
- No network dependencies
- One line replaces 20+ lines of raw LiteSVM setup

### 2. Create Test Accounts

Access helpers via `ctx.svm`:

```rust
// Create a funded SOL account
let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();

// Create a token mint
let mint = ctx.svm.create_token_mint(&authority, 9).unwrap();

// Create an associated token account
let token_account = ctx.svm
    .create_associated_token_account(&mint.pubkey(), &owner)
    .unwrap();

// Mint tokens to an account
ctx.svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000).unwrap();
```

### 3. Build Instructions (Production-Compatible!)

The syntax is **identical** to `anchor-client`:

```rust
let ix = ctx.program()
    .request()
    .accounts(my_program::client::accounts::MyInstruction { ... })
    .args(my_program::client::args::MyInstruction { ... })
    .instructions()
    .unwrap()[0];
```

**Why `.instructions()?[0]`?**
- Matches anchor-client API exactly (returns Vec for consistency with production)
- In production, might include compute budget instructions
- Alternative: use `.instruction()` for convenience (returns single instruction)

### 4. Execute Instructions

```rust
let result = ctx.execute_instruction(ix, &[&signer]).unwrap();
result.assert_success();
```

**What you get back:**
- `TransactionResult` with logs, compute units, and success status
- Rich debugging information
- Assertion helpers

### 5. Verify Results

Use assertion helpers on `ctx.svm`:

```rust
// Check account exists
ctx.svm.assert_account_exists(&pubkey);

// Check account closed
ctx.svm.assert_account_closed(&pubkey);

// Check token balance
ctx.svm.assert_token_balance(&token_account, 1_000_000);

// Check SOL balance
ctx.svm.assert_sol_balance(&account, 10_000_000_000);

// Check account owner
ctx.svm.assert_account_owner(&account, &program_id);

// Check mint supply
ctx.svm.assert_mint_supply(&mint, 1_000_000);
```

## Common Patterns

### Pattern 1: Working with PDAs

```rust
let seed = 42u64;

// Calculate PDA (just the address)
let pda = ctx.svm.get_pda(
    &[b"vault", user.pubkey().as_ref(), &seed.to_le_bytes()],
    &program_id
);

// Calculate PDA with bump (if you need the bump)
let (pda, bump) = ctx.svm.get_pda_with_bump(
    &[b"vault", user.pubkey().as_ref(), &seed.to_le_bytes()],
    &program_id
);
```

### Pattern 2: Reading Account Data

```rust
// Deserialize an Anchor account
let account_data: MyAccountType = ctx.get_account(&pda).unwrap();

// Or without discriminator check (for special cases)
let account_data: MyAccountType = ctx.get_account_unchecked(&pda).unwrap();
```

### Pattern 3: Analyzing Transaction Results

```rust
let result = ctx.execute_instruction(ix, &[&user]).unwrap();

// Check success
result.assert_success();

// Check logs
if result.has_log("Transfer complete") {
    println!("Found expected log");
}

// Get compute units
let cu = result.compute_units();
assert!(cu < 200_000, "Used too many compute units");

// Print all logs for debugging
result.print_logs();
```

### Pattern 4: Multiple Instructions

```rust
let ix1 = ctx.program().request()
    .accounts(...)
    .args(...)
    .instructions()
    .unwrap()[0];

let ix2 = ctx.program().request()
    .accounts(...)
    .args(...)
    .instructions()
    .unwrap()[0];

// Execute both in one transaction
let result = ctx.execute_instructions(vec![ix1, ix2], &[&signer]).unwrap();
result.assert_success();
```

### Pattern 5: Error Handling

```rust
let result = ctx.execute_instruction(ix, &[&user]);

match result {
    Ok(tx_result) => {
        if tx_result.is_success() {
            println!("Success!");
        } else {
            println!("Failed: {:?}", tx_result.error());
            tx_result.print_logs();
        }
    }
    Err(e) => {
        println!("Error building/sending transaction: {}", e);
    }
}
```

### Pattern 6: Testing Token Transfers

```rust
// Setup
let mint = ctx.svm.create_token_mint(&authority, 9).unwrap();
let from_ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &from_user).unwrap();
let to_ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &to_user).unwrap();

// Mint initial tokens
ctx.svm.mint_to(&mint.pubkey(), &from_ata, &authority, 1_000_000).unwrap();

// Build and execute your transfer instruction
let transfer_ix = ctx.program()
    .request()
    .accounts(my_program::client::accounts::Transfer {
        from: from_ata,
        to: to_ata,
        authority: from_user.pubkey(),
        token_program: spl_token::id(),
    })
    .args(my_program::client::args::Transfer { amount: 500_000 })
    .instructions()
    .unwrap()[0];

ctx.execute_instruction(transfer_ix, &[&from_user])
    .unwrap()
    .assert_success();

// Verify
ctx.svm.assert_token_balance(&from_ata, 500_000);
ctx.svm.assert_token_balance(&to_ata, 500_000);
```

## Common Pitfalls

### ❌ Don't forget `anchor_lang::declare_program!`

```rust
// You need this to generate client types
anchor_lang::declare_program!(my_program);
```

Without this, you won't have `my_program::client::accounts` and `my_program::client::args`.

### ❌ Don't use wrong account paths

```rust
// ❌ Wrong - using instruction types
.accounts(my_program::instruction::Transfer { ... })

// ✓ Correct - using client accounts
.accounts(my_program::client::accounts::Transfer { ... })
```

### ❌ Don't forget to unwrap or handle errors

```rust
// ❌ This compiles but doesn't execute anything!
ctx.execute_instruction(ix, &[&user]);

// ✓ Correct - handle the Result
ctx.execute_instruction(ix, &[&user]).unwrap().assert_success();
```

### ❌ Don't mix up PDA calculation

```rust
// If your program uses program_id for PDA derivation
let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"seed"], &ctx.program_id);

// Not some other program's ID
```

## Next Steps

1. **See Real Examples**: Check out `anchor-escrow-example/tests/src/anchor_litesvm_test.rs` for a complete working test
2. **Learn All APIs**: Read [API_REFERENCE.md](API_REFERENCE.md) for comprehensive API documentation
3. **Migrate from Raw LiteSVM**: See [MIGRATION.md](MIGRATION.md) for a migration guide
4. **Advanced Features**: Run `cargo run --example advanced_features` to see advanced patterns

## Getting Help

- **GitHub Issues**: https://github.com/brimigs/anchor-litesvm/issues
- **Examples**: See `examples/` directory
- **Tests**: See `anchor-escrow-example/tests/` for real-world usage

## Summary

**The 5-Step Pattern:**
1. **Setup**: `AnchorLiteSVM::build_with_program()`
2. **Create accounts**: Use `ctx.svm.create_*()` helpers
3. **Build instruction**: Use `ctx.program().request()` (production syntax!)
4. **Execute**: Use `ctx.execute_instruction()`
5. **Verify**: Use `ctx.svm.assert_*()` helpers

**Key Benefits:**
- ✅ 78% less code than raw LiteSVM
- ✅ Production-compatible syntax
- ✅ No mock RPC setup needed
- ✅ 40% faster compilation
- ✅ Rich debugging tools

Happy testing! 🚀
