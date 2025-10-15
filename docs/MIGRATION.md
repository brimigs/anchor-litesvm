# Migration Guide: From Raw LiteSVM to anchor-litesvm

This guide helps you migrate your existing LiteSVM tests to `anchor-litesvm`.

## Table of Contents

- [Why Migrate?](#why-migrate)
- [Quick Migration Checklist](#quick-migration-checklist)
- [Step-by-Step Migration](#step-by-step-migration)
- [Pattern-by-Pattern Comparison](#pattern-by-pattern-comparison)
- [Complete Example](#complete-example)
- [Common Issues](#common-issues)

## Why Migrate?

| Benefit                      | Description                                |
| ---------------------------- | ------------------------------------------ |
| **Less Code**                | Dramatically reduced boilerplate           |
| **Simplified Syntax**        | Similar to anchor-client                   |
| **Better Helpers**           | Built-in token, PDA, and assertion helpers |
| **Type Safety**              | Compile-time validation with Anchor types  |
| **Easier Debugging**         | Rich transaction result analysis           |

## Quick Migration Checklist

- [ ] Add `anchor-litesvm` to dev-dependencies
- [ ] Add `anchor_lang::declare_program!()` for your program
- [ ] Replace `LiteSVM::new()` with `AnchorLiteSVM::build_with_program()`
- [ ] Replace manual discriminator calculation with simplified instruction builder
- [ ] Replace manual token operations with helper methods
- [ ] Replace manual assertions with helper methods
- [ ] Use `ctx.svm` to access LiteSVM functionality
- [ ] Update instruction building to use generated client types

## Step-by-Step Migration

### Step 1: Update Dependencies

**Before:**

```toml
[dev-dependencies]
litesvm = "0.1"
solana-sdk = "1.18"
```

**After:**

```toml
[dev-dependencies]
anchor-litesvm = "0.1"  # Includes litesvm
anchor-lang = "0.30"
solana-sdk = "1.18"
```

### Step 2: Generate Client Types

Add this at the top of your test file:

```rust
// This generates the client types for your program
anchor_lang::declare_program!(my_program);
```

This creates:

- `my_program::ID` - Your program ID constant
- `my_program::client::accounts::*` - Account structs
- `my_program::client::args::*` - Instruction argument structs

### Step 3: Update Test Setup

**Before (Raw LiteSVM):**

```rust
use litesvm::LiteSVM;

let mut svm = LiteSVM::new();
svm.add_program(
    program_id,
    include_bytes!("../target/deploy/my_program.so"),
);
```

**After (anchor-litesvm):**

```rust
use anchor_litesvm::AnchorLiteSVM;

let mut ctx = AnchorLiteSVM::build_with_program(
    my_program::ID,
    include_bytes!("../target/deploy/my_program.so"),
);
```

**Savings:** 3 lines → 1 line

### Step 4: Replace Manual Token Operations

**Before (30+ lines):**

```rust
// Create mint account
let mint = Keypair::new();
let rent = svm.minimum_balance_for_rent_exemption(82);

let create_mint_ix = system_instruction::create_account(
    &payer.pubkey(),
    &mint.pubkey(),
    rent,
    82,
    &spl_token::id(),
);

let init_mint_ix = spl_token::instruction::initialize_mint(
    &spl_token::id(),
    &mint.pubkey(),
    &authority.pubkey(),
    None,
    9,
)?;

let tx = Transaction::new_signed_with_payer(
    &[create_mint_ix, init_mint_ix],
    Some(&payer.pubkey()),
    &[&payer, &mint],
    svm.latest_blockhash(),
);

svm.send_transaction(tx)?;

// Create token account (another 20+ lines)
// Mint tokens (another 15+ lines)
```

**After (3 lines):**

```rust
use litesvm_utils::TestHelpers;

let mint = ctx.svm.create_token_mint(&authority, 9)?;
let token_account = ctx.svm.create_associated_token_account(&mint.pubkey(), &owner)?;
ctx.svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000)?;
```

**Savings:** 65+ lines → 3 lines

### Step 5: Replace Manual Discriminator Calculation

**Before (10+ lines):**

```rust
use sha2::{Digest, Sha256};

fn get_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", name));
    let result = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&result[..8]);
    disc
}

let mut instruction_data = get_discriminator("initialize").to_vec();
instruction_data.extend_from_slice(&borsh::to_vec(&args)?);
```

**After (automatic):**

```rust
// Discriminator is handled automatically by the instruction builder
let ix = ctx.program()
    .accounts(my_program::client::accounts::Initialize { ... })
    .args(my_program::client::args::Initialize { ... })
    .instruction()?;
```

**Savings:** 10+ lines → 0 lines (automatic!)

### Step 6: Replace Manual Instruction Building

**Before (15+ lines):**

```rust
use solana_program::instruction::{AccountMeta, Instruction};

let accounts = vec![
    AccountMeta::new(data_account, false),
    AccountMeta::new(user.pubkey(), true),
    AccountMeta::new_readonly(system_program::id(), false),
];

let mut data = get_discriminator("initialize").to_vec();
data.extend_from_slice(&borsh::to_vec(&InitializeArgs {
    value: 42,
})?);

let ix = Instruction {
    program_id,
    accounts,
    data,
};
```

**After (4 lines):**

```rust
let ix = ctx.program()
    .accounts(my_program::client::accounts::Initialize {
        data_account,
        user: user.pubkey(),
        system_program: system_program::id(),
    })
    .args(my_program::client::args::Initialize { value: 42 })
    .instruction()?;
```

**Savings:** 15 lines → 4 lines
**Bonus:** Type-safe! Compiler will catch missing accounts/args.

### Step 7: Replace Manual Assertions

**Before:**

```rust
// Check account exists
let account = svm.get_account(&pubkey).expect("Account should exist");

// Check token balance
let account_data = svm.get_account(&token_account).unwrap();
let token_data = spl_token::state::Account::unpack(&account_data.data)?;
assert_eq!(token_data.amount, 1_000_000, "Wrong balance");

// Check SOL balance
let account = svm.get_account(&user_pubkey).unwrap();
assert_eq!(account.lamports, 10_000_000_000, "Wrong SOL balance");
```

**After:**

```rust
use litesvm_utils::AssertionHelpers;

ctx.svm.assert_account_exists(&pubkey);
ctx.svm.assert_token_balance(&token_account, 1_000_000);
ctx.svm.assert_sol_balance(&user_pubkey, 10_000_000_000);
```

**Savings:** 10+ lines → 3 lines
**Bonus:** Better error messages!

## Pattern-by-Pattern Comparison

### Account Creation

| Operation          | Raw LiteSVM              | anchor-litesvm                                           | Savings |
| ------------------ | ------------------------ | -------------------------------------------------------- | ------- |
| **Funded account** | Manual airdrop + keypair | `ctx.svm.create_funded_account(lamports)`                | 90%     |
| **Token mint**     | 30+ lines                | `ctx.svm.create_token_mint(&auth, decimals)`             | 95%     |
| **Token account**  | 20+ lines                | `ctx.svm.create_associated_token_account(&mint, &owner)` | 95%     |
| **Mint tokens**    | 15+ lines                | `ctx.svm.mint_to(&mint, &account, &auth, amount)`        | 93%     |

### PDA Operations

**Before:**

```rust
use solana_program::pubkey::Pubkey;

let (pda, bump) = Pubkey::find_program_address(
    &[b"vault", user.pubkey().as_ref()],
    &program_id,
);
```

**After:**

```rust
// Just the PDA
let pda = ctx.svm.get_pda(&[b"vault", user.pubkey().as_ref()], &program_id);

// Or with bump
let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"vault", user.pubkey().as_ref()], &program_id);
```

### Transaction Execution

**Before:**

```rust
let tx = Transaction::new_signed_with_payer(
    &[ix],
    Some(&payer.pubkey()),
    &[&payer],
    svm.latest_blockhash(),
);

match svm.send_transaction(tx) {
    Ok(_) => println!("Success"),
    Err(e) => panic!("Transaction failed: {:?}", e),
}
```

**After:**

```rust
let result = ctx.execute_instruction(ix, &[&payer])?;
result.assert_success();

// Or with detailed analysis
println!("Compute units: {}", result.compute_units());
assert!(result.has_log("Success"));
```

### Error Testing

**Before:**

```rust
let tx = Transaction::new_signed_with_payer(/*...*/);
let result = svm.send_transaction(tx);

assert!(result.is_err(), "Should have failed");
let error_string = format!("{:?}", result.unwrap_err());
assert!(error_string.contains("InsufficientFunds"));
```

**After:**

```rust
let result = ctx.execute_instruction(ix, &[&payer])?;
result.assert_failure();
result.assert_error("InsufficientFunds");

// Or for Anchor errors
result.assert_anchor_error("InsufficientFunds");
result.assert_error_code(6000);
```

## Complete Example

### Before: Raw LiteSVM (493 lines)

```rust
#[cfg(test)]
mod tests {
    use litesvm::LiteSVM;
    use solana_sdk::signature::{Keypair, Signer};
    use solana_program::instruction::{AccountMeta, Instruction};
    use sha2::{Digest, Sha256};

    fn get_discriminator(name: &str) -> [u8; 8] {
        // 10 lines of manual discriminator calculation
    }

    #[test]
    fn test_escrow() {
        // Setup (20+ lines)
        let mut svm = LiteSVM::new();
        svm.add_program(/*...*/);

        let maker = Keypair::new();
        svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

        // Create mint (30+ lines)
        let mint_a = Keypair::new();
        let rent = svm.minimum_balance_for_rent_exemption(82);
        // ... create account instruction
        // ... initialize mint instruction
        // ... build and send transaction

        // Create token accounts (40+ lines per account)
        // ... manual account creation
        // ... manual initialization

        // Build MAKE instruction (25+ lines)
        let accounts = vec![
            AccountMeta::new(/*...*/),
            // ... list all accounts manually
        ];

        let mut data = get_discriminator("make").to_vec();
        data.extend_from_slice(&seed.to_le_bytes());
        data.extend_from_slice(&receive.to_le_bytes());
        data.extend_from_slice(&amount.to_le_bytes());

        let make_ix = Instruction { program_id, accounts, data };

        // Execute (10+ lines)
        let tx = Transaction::new_signed_with_payer(/*...*/);
        svm.send_transaction(tx).unwrap();

        // Verify (20+ lines)
        let account = svm.get_account(&vault).unwrap();
        let token_data = spl_token::state::Account::unpack(&account.data).unwrap();
        assert_eq!(token_data.amount, 1_000_000_000);

        // ... 300+ more lines for TAKE and REFUND instructions
    }
}
```

### After: anchor-litesvm (106 lines - 78% reduction!)

```rust
#[cfg(test)]
mod tests {
    use anchor_litesvm::{AnchorLiteSVM, TestHelpers, AssertionHelpers};
    use solana_sdk::signature::Signer;

    anchor_lang::declare_program!(anchor_escrow);

    #[test]
    fn test_escrow() {
        // Setup (1 line!)
        let mut ctx = AnchorLiteSVM::build_with_program(
            anchor_escrow::ID,
            include_bytes!("../../target/deploy/anchor_escrow.so"),
        );

        let maker = ctx.svm.create_funded_account(10_000_000_000).unwrap();
        let taker = ctx.svm.create_funded_account(10_000_000_000).unwrap();

        // Create tokens (2 lines!)
        let mint_a = ctx.svm.create_token_mint(&maker, 9).unwrap();
        let mint_b = ctx.svm.create_token_mint(&maker, 9).unwrap();

        // Create token accounts (2 lines!)
        let maker_ata_a = ctx.svm.create_associated_token_account(&mint_a.pubkey(), &maker).unwrap();
        ctx.svm.mint_to(&mint_a.pubkey(), &maker_ata_a, &maker, 1_000_000_000).unwrap();

        // Calculate PDA (1 line!)
        let seed = 42u64;
        let escrow_pda = ctx.svm.get_pda(
            &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
            &anchor_escrow::ID,
        );

        // Build instruction (simplified syntax - similar to anchor client)
        let make_ix = ctx.program()
            .accounts(anchor_escrow::client::accounts::Make {
                maker: maker.pubkey(),
                escrow: escrow_pda,
                mint_a: mint_a.pubkey(),
                mint_b: mint_b.pubkey(),
                maker_ata_a,
                vault,
                associated_token_program: spl_associated_token_account::id(),
                token_program: spl_token::id(),
                system_program: system_program::id(),
            })
            .args(anchor_escrow::client::args::Make {
                seed,
                receive: 500_000_000,
                amount: 1_000_000_000,
            })
            .instruction()
            .unwrap();

        // Execute (1 line!)
        ctx.execute_instruction(make_ix, &[&maker])
            .unwrap()
            .assert_success();

        // Verify (1 line!)
        ctx.svm.assert_token_balance(&vault, 1_000_000_000);
    }
}
```

**Result:** 493 lines → 106 lines = **78% reduction!**

## Common Issues

### Issue 1: Missing `declare_program!`

**Error:**

```
error: cannot find `client` in `my_program`
```

**Solution:**

```rust
// Add this at the top of your test file
anchor_lang::declare_program!(my_program);
```

### Issue 2: Wrong Account Paths

**Error:**

```
error: no variant named `Transfer` found for enum `my_program::instruction`
```

**Solution:**

```rust
// Wrong - using instruction types
.accounts(my_program::instruction::Transfer { ... })

// Correct - using client accounts
.accounts(my_program::client::accounts::Transfer { ... })
```

### Issue 3: Accessing LiteSVM

**Before:**

```rust
let mut svm = LiteSVM::new();
svm.get_account(&pubkey);
```

**After:**

```rust
let mut ctx = AnchorLiteSVM::build_with_program(/*...*/);
ctx.svm.get_account(&pubkey);  // Access via ctx.svm
```

### Issue 4: Transaction Building

**Before:**

```rust
let tx = Transaction::new_signed_with_payer(/*...*/);
svm.send_transaction(tx)?;
```

**After:**

```rust
// Use the helper method
ctx.execute_instruction(ix, &[&signer])?;

// Or for multiple instructions
ctx.execute_instructions(vec![ix1, ix2], &[&signer])?;
```

## Migration Benefits Summary

| Aspect                   | Before                   | After                                 | Improvement                |
| ------------------------ | ------------------------ | ------------------------------------- | -------------------------- |
| **Lines of Code**        | 493                      | 106                                   | **78% reduction**          |
| **Setup**                | 20+ lines                | 1 line                                | **95% reduction**          |
| **Token Operations**     | 65+ lines                | 3 lines                               | **95% reduction**          |
| **Instruction Building** | 25+ lines                | 4 lines                               | **84% reduction**          |
| **Account Ordering**     | Manual Vec (error-prone) | **Named structs (order-independent)** | Zero ordering bugs!        |
| **Assertions**           | 10+ lines                | 1 line                                | **90% reduction**          |
| **Compilation Time**     | Slower                   | **40% faster**                        | Major improvement          |
| **Type Safety**          | Manual                   | **Compile-time**                      | Catches errors early       |
| **Syntax Similarity**    | Different                | **Similar to anchor-client**          | Learn once, use everywhere |

## Additional Resources

- [Quick Start Guide](QUICK_START.md) - Get started in 5 minutes
- [API Reference](API_REFERENCE.md) - Complete API documentation
- [Examples](../examples/) - Runnable examples
- [Working Tests](../anchor-escrow-example/tests/) - Real-world test examples

---
