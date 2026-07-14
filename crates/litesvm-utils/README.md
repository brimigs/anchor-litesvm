# litesvm-utils

**Framework-agnostic testing utilities for LiteSVM** - One-line token operations, assertions, and account management for any Solana program.

[![Crates.io](https://img.shields.io/crates/v/litesvm-utils.svg)](https://crates.io/crates/litesvm-utils)
[![Documentation](https://docs.rs/litesvm-utils/badge.svg)](https://docs.rs/litesvm-utils)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`litesvm-utils` provides ergonomic testing utilities that work with **any Solana program** - Native, Anchor, SPL, or custom frameworks. It eliminates boilerplate by providing one-line helpers for common testing operations.

**Use this crate if you're testing:**
- Native Solana programs
- SPL programs
- Non-Anchor custom frameworks
- Or if you need just testing utilities without Anchor-specific features

> **Note:** If you're testing Anchor programs, consider using [`anchor-litesvm`](https://crates.io/crates/anchor-litesvm) instead, which builds on this crate and adds Anchor-specific features.

## Installation

```toml
[dev-dependencies]
litesvm-utils = "0.4"
```

## Quick Start

```rust
use litesvm_utils::{LiteSVMBuilder, TestHelpers, AssertionHelpers, TransactionHelpers};
use litesvm_utils::Signer;

#[test]
fn test_my_program() {
    // 1. Setup with your program
    let mut svm = LiteSVMBuilder::build_with_program(program_id, &program_bytes);

    // 2. Create test accounts (one line each!)
    let payer = svm.create_funded_account(10_000_000_000).unwrap();

    // 3. Token operations (one line each!)
    let mint = svm.create_token_mint(&payer, 9).unwrap();
    let token_account = svm.create_associated_token_account(&mint.pubkey(), &payer).unwrap();
    svm.mint_to(&mint.pubkey(), &token_account, &payer, 1_000_000).unwrap();

    // 4. Execute your instruction
    let ix = your_program_instruction(...);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // 5. Rich result analysis
    result.assert_success();
    assert!(result.compute_units() < 200_000);
    assert!(result.has_log("Success"));

    // 6. State assertions
    svm.assert_token_balance(&token_account, 1_000_000);
    svm.assert_account_exists(&some_pda);
}
```

## Features

### Test Account Helpers

```rust
// Create funded accounts
let user = svm.create_funded_account(10_000_000_000)?;
let accounts = svm.create_funded_accounts(5, 1_000_000_000)?;
```

### Token Operations

```rust
// Create mint and token accounts
let mint = svm.create_token_mint(&authority, 9)?;
let token_account = svm.create_token_account(&mint.pubkey(), &owner)?;
let ata = svm.create_associated_token_account(&mint.pubkey(), &owner)?;

// Mint tokens
svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000)?;
```

### Transaction Helpers

```rust
// Send single instruction
let result = svm.send_instruction(ix, &[&signer])?;

// Send multiple instructions
let result = svm.send_instructions(&[ix1, ix2], &[&signer])?;

// Analyze results
result.assert_success();
result.assert_failure();
result.assert_error("InsufficientFunds");

// Debug
result.print_logs();
let cu = result.compute_units();
let logs = result.logs();
```

### Assertions

```rust
// Account existence
svm.assert_account_exists(&pubkey);
svm.assert_account_closed(&pubkey);

// Ownership and data
svm.assert_account_owner(&account, &program_id);
svm.assert_account_data_len(&account, 165);

// Balances
svm.assert_token_balance(&token_account, 1_000_000);
svm.assert_sol_balance(&account, 10_000_000_000);
svm.assert_mint_supply(&mint, 1_000_000);
```

### PDA Utilities

```rust
// Get PDA
let pda = svm.get_pda(&[b"seed", user.as_ref()], &program_id);

// Get PDA with bump
let (pda, bump) = svm.get_pda_with_bump(&[b"seed"], &program_id);

// Derive (alias)
let pda = svm.derive_pda(&[b"seed"], &program_id);
```

### Clock Manipulation

Two flavors, depending on whether your program reads the Clock sysvar's
`slot` or its `unix_timestamp`:

```rust
// Slot-based (for slot-anchored constraints)
let slot = svm.get_current_slot();
svm.advance_slot(100);

// Timestamp-based (for unix_timestamp constraints:
// escrow expiries, vesting cliffs, etc.)
let now = svm.get_unix_timestamp();
svm.warp_to_timestamp(1_700_000_000); // absolute set
svm.advance_seconds(3_600);           // +1 hour
svm.advance_days(30);                 // +30 days
```

The timestamp helpers set the Clock sysvar's `unix_timestamp` directly and
leave other fields (slot, epoch, etc.) alone, so they compose with the
slot-based helpers above.

## API Reference

### Traits

| Trait | Description |
|-------|-------------|
| `TestHelpers` | Account creation, token operations, PDA utilities |
| `AssertionHelpers` | State verification and balance assertions |
| `TransactionHelpers` | Transaction execution and result handling |
| `ProgramTestExt` | Program deployment helpers |

### Builder

```rust
// New empty SVM
let svm = LiteSVMBuilder::new().build();

// With single program
let svm = LiteSVMBuilder::build_with_program(program_id, &bytes);

// With multiple programs
let svm = LiteSVMBuilder::build_with_programs(&[
    (program1_id, &program1_bytes),
    (program2_id, &program2_bytes),
]);

// Chained building
let svm = LiteSVMBuilder::new()
    .deploy_program(program_id, &bytes)
    .build();
```

## Testing

This crate has comprehensive test coverage:

```bash
cargo test -p litesvm-utils    # 56 tests
```

## Related Crates

- [`anchor-litesvm`](https://crates.io/crates/anchor-litesvm) - Anchor-specific testing (builds on this crate)
- [`litesvm`](https://crates.io/crates/litesvm) - The underlying fast Solana VM

## License

MIT License - see [LICENSE](../../LICENSE) for details.
