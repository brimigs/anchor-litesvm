# anchor-litesvm

This is a crate for writing LiteSVM tests when you are testing an Anchor program.

[![Crates.io](https://img.shields.io/crates/v/anchor-litesvm.svg)](https://crates.io/crates/anchor-litesvm)

<!-- [![Documentation](https://docs.rs/anchor-litesvm/badge.svg)](https://docs.rs/anchor-litesvm)
[![License](https://img.shields.io/crates/l/anchor-litesvm.svg)](https://github.com/anchor-litesvm/anchor-litesvm#license) -->

## Overview

`anchor-litesvm` provides a minimal wrapper around LiteSVM that handles Anchor-specific patterns, reducing test code by 60-70% while maintaining full control and flexibility.

## Quick Start

```rust
use anchor_litesvm::AnchorLiteSVM;

let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

// access helper methods from the Anchor Context
let account = ctx.create_funded_account(10_000_000_000)?;
let mint = ctx.create_token_mint(&account, 9)?;
```

## Why Use anchor-litesvm?

### 1. Instruction Builder with Direct Execution

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

let make_instruction = Instruction {
    program_id,
    accounts: vec![
        AccountMeta::new(maker.pubkey(), true),  // maker
        AccountMeta::new(escrow_pda, false),      // escrow
        AccountMeta::new_readonly(mint_a.pubkey(), false), // mint_a
        AccountMeta::new_readonly(mint_b.pubkey(), false), // mint_b
        AccountMeta::new(maker_ata_a, false),     // maker_ata_a
        AccountMeta::new(vault, false),           // vault
        AccountMeta::new_readonly(spl_associated_token_account::id(), false), // associated_token_program
        AccountMeta::new_readonly(spl_token::id(), false), // token_program
        AccountMeta::new_readonly(system_program::id(), false), // system_program
    ],
    data: instruction_data,
};

let tx = Transaction::new_signed_with_payer(
    &[make_instruction],
    Some(&maker.pubkey()),
    &[&maker],
    svm.latest_blockhash(),
);

let result = svm.send_transaction(tx);

// After (with anchor-litesvm):
// Build and execute in one call
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
