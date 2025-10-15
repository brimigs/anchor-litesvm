# Quick Start Guide

Get started with `anchor-litesvm` in 5 minutes! This guide walks you through setting up your first test with simplified syntax - similar to anchor-client.

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
    // 1. Setup: One-line initialization
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
    // 3. Build Instruction: Simplified syntax
    // ========================================
    let ix = ctx.program()
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
        .instruction()
        .unwrap();

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

**What it does:** Creates a test environment with your program deployed. That's it

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

### 3. Build Instructions (Simplified Syntax - Similar to Anchor Client)

The syntax is similar to anchor-client:

```rust
let ix = ctx.program()
    .accounts(my_program::client::accounts::MyInstruction { ... })
    .args(my_program::client::args::MyInstruction { ... })
    .instruction()
    .unwrap();
```

**Why `.instruction()`**
- Clean and direct - no RPC layer abstractions needed for testing
- Returns single instruction directly
- Similar pattern to anchor-client for easy knowledge transfer

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

## No More Account Ordering Headaches

This is the number one pain point in Solana testing - and anchor-litesvm eliminates it completely.

### The Problem with Raw LiteSVM

In raw Solana/LiteSVM testing, you must manually build instruction accounts as a `Vec<AccountMeta>` in the exact order your program expects. Get the order wrong and your transaction will fail, or worse - it might succeed but use the wrong accounts.

```rust
// Raw LiteSVM - You MUST get the order exactly right
let instruction = Instruction {
    program_id,
    accounts: vec![
        AccountMeta::new(maker.pubkey(), true),  // Position 0
        AccountMeta::new(escrow_pda, false),      // Position 1
        AccountMeta::new_readonly(mint_a, false), // Position 2
        AccountMeta::new_readonly(mint_b, false), // Position 3
        AccountMeta::new(maker_ata_a, false),     // Position 4
        AccountMeta::new(vault, false),           // Position 5
        // ... 3 more accounts in exact order
    ],
    data: instruction_data,
};

// If you accidentally swap positions 4 and 5, your transaction fails
// Or worse - it might use the wrong accounts and succeed with bad state
```

**Common bugs from account ordering:**
- Swapping two accounts → transaction fails with "invalid account"
- Missing an account → off-by-one errors in all subsequent positions
- Adding a new account → must find correct position or everything breaks
- Program changes account order → all tests need manual updates

### How anchor-litesvm Solves This

With anchor-litesvm, you use named struct fields instead of ordered vectors. The order does not matter - Anchor's `ToAccountMetas` trait handles the correct ordering automatically.

```rust
// anchor-litesvm - Order does not matter, just fill in the fields
let ix = ctx.program()
    .accounts(my_program::client::accounts::Make {
        // You can put these fields in ANY order you want
        vault,                    // Field 1
        maker: maker.pubkey(),   // Field 2
        escrow: escrow_pda,       // Field 3
        maker_ata_a,              // Field 4
        mint_b: mint_b.pubkey(),  // Field 5 (swapped)
        mint_a: mint_a.pubkey(),  // Field 6 (swapped)
        // The generated struct ensures they are passed in the correct order
        system_program: solana_sdk::system_program::id(),
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
    })
    .args(my_program::client::args::Make {
        seed: 42,
        receive: 500_000_000,
        amount: 1_000_000_000,
    })
    .instruction()?;

// Reorder fields however you want - it just works
```

### Benefits

1. **Type Safety**: Compiler ensures all required accounts are provided
2. **Named Fields**: Clear what each account is for (no guessing positions)
3. **Order Independence**: Rearrange fields however you like
4. **Refactor Safe**: If program changes account order, tests won't compile until fixed
5. **IDE Support**: Autocomplete shows all required account fields
6. **Less Error-Prone**: No manual Vec construction means fewer bugs

### How It Works Under the Hood

```rust
// When you use declare_program, it generates account structs like this:
#[derive(Accounts)]
pub struct Make {
    pub maker: Pubkey,
    pub escrow: Pubkey,
    // ... more fields
}

// The generated struct implements ToAccountMetas, which knows
// the correct ordering from your program definition:
impl ToAccountMetas for Make {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        // Automatically creates Vec in the CORRECT order!
        vec![
            AccountMeta::new(self.maker, true),
            AccountMeta::new(self.escrow, false),
            // ... in program-defined order
        ]
    }
}
```

When you call `.accounts(my_struct)`, the `InstructionBuilder` calls `.to_account_metas()` internally, which handles the ordering for you automatically

### Real-World Example

Here's a complete example showing the difference:

```rust
// Raw LiteSVM: 15 lines, error-prone
let accounts = vec![
    AccountMeta::new(taker.pubkey(), true),
    AccountMeta::new(maker.pubkey(), false),
    AccountMeta::new(escrow_pda, false),
    AccountMeta::new_readonly(mint_a, false),
    AccountMeta::new_readonly(mint_b, false),
    AccountMeta::new(vault, false),
    AccountMeta::new(taker_ata_a, false),
    AccountMeta::new(taker_ata_b, false),
    AccountMeta::new(maker_ata_b, false),
    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
    AccountMeta::new_readonly(spl_token::id(), false),
    AccountMeta::new_readonly(system_program::id(), false),
];
let ix = Instruction { program_id, accounts, data };

// anchor-litesvm: Clear, self-documenting, order-independent
let ix = ctx.program()
    .accounts(anchor_escrow::client::accounts::Take {
        taker: taker.pubkey(),
        maker: maker.pubkey(),
        escrow: escrow_pda,
        mint_a: mint_a.pubkey(),
        mint_b: mint_b.pubkey(),
        vault,
        taker_ata_a,
        taker_ata_b,
        maker_ata_b,
        associated_token_program: spl_associated_token_account::id(),
        token_program: spl_token::id(),
        system_program: system_program::id(),
    })
    .args(anchor_escrow::client::args::Take {})
    .instruction()?;
```

**Result**: Named fields are easier to read, maintain, and refactor. No more account ordering bugs

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
let ix1 = ctx.program()
    .accounts(...)
    .args(...)
    .instruction()
    .unwrap();

let ix2 = ctx.program()
    .accounts(...)
    .args(...)
    .instruction()
    .unwrap();

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
    .accounts(my_program::client::accounts::Transfer {
        from: from_ata,
        to: to_ata,
        authority: from_user.pubkey(),
        token_program: spl_token::id(),
    })
    .args(my_program::client::args::Transfer { amount: 500_000 })
    .instruction()
    .unwrap();

ctx.execute_instruction(transfer_ix, &[&from_user])
    .unwrap()
    .assert_success();

// Verify
ctx.svm.assert_token_balance(&from_ata, 500_000);
ctx.svm.assert_token_balance(&to_ata, 500_000);
```

## Common Pitfalls

### Do not forget `anchor_lang::declare_program`

```rust
// You need this to generate client types
anchor_lang::declare_program!(my_program);
```

Without this, you will not have `my_program::client::accounts` and `my_program::client::args`.

### Do not use wrong account paths

```rust
// Wrong - using instruction types
.accounts(my_program::instruction::Transfer { ... })

// Correct - using client accounts
.accounts(my_program::client::accounts::Transfer { ... })
```

### Do not forget to unwrap or handle errors

```rust
// This compiles but does not execute anything
ctx.execute_instruction(ix, &[&user]);

// Correct - handle the Result
ctx.execute_instruction(ix, &[&user]).unwrap().assert_success();
```

### Do not mix up PDA calculation

```rust
// If your program uses program_id for PDA derivation
let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"seed"], &ctx.program_id);

// Not some other program's ID
```

### Do not worry about account ordering

This is a non-issue with anchor-litesvm, but it is the number one bug source in raw LiteSVM:

```rust
// In raw LiteSVM - Order matters
let accounts = vec![
    AccountMeta::new(maker.pubkey(), true),   // Position 0
    AccountMeta::new(escrow_pda, false),       // Position 1
    // If you swap these two, your tx fails
    AccountMeta::new_readonly(mint_a, false),  // Position 2
    AccountMeta::new_readonly(mint_b, false),  // Position 3
];

// In anchor-litesvm - Order does not matter
.accounts(my_program::client::accounts::Make {
    // Put fields in ANY order - it just works
    mint_b: mint_b.pubkey(),  // Can be first
    maker: maker.pubkey(),     // Or here
    escrow: escrow_pda,        // Order does not matter
    mint_a: mint_a.pubkey(),   // Named fields
})
```

**Why this matters:** In raw testing, swapping two accounts in a Vec causes runtime failures. With anchor-litesvm's named struct fields, the compiler ensures correct ordering automatically

## Next Steps

1. **See Real Examples**: Check out [anchor-escrow-with-litesvm](https://github.com/brimigs/anchor-escrow-with-litesvm) for a complete working test
2. **Learn All APIs**: Read [API_REFERENCE.md](API_REFERENCE.md) for comprehensive API documentation
3. **Migrate from Raw LiteSVM**: See [MIGRATION.md](MIGRATION.md) for a migration guide
4. **Advanced Features**: Run `cargo run --example advanced_features` to see advanced patterns

## Getting Help

- **GitHub Issues**: https://github.com/brimigs/anchor-litesvm/issues
- **Examples**: See `examples/` directory
- **Tests**: See [anchor-escrow-with-litesvm](https://github.com/brimigs/anchor-escrow-with-litesvm) for real-world usage

## Summary

**The 5-Step Pattern:**
1. **Setup**: `AnchorLiteSVM::build_with_program()`
2. **Create accounts**: Use `ctx.svm.create_*()` helpers
3. **Build instruction**: Use `ctx.program().accounts().args().instruction()` (simplified syntax)
4. **Execute**: Use `ctx.execute_instruction()`
5. **Verify**: Use `ctx.svm.assert_*()` helpers

**Key Benefits:**
- 78% less code than raw LiteSVM
- Simplified syntax similar to anchor-client
- No mock RPC setup needed
- 40% faster compilation
- Rich debugging tools

Happy testing
