# anchor-litesvm

**Simplified Anchor testing with LiteSVM** - Similar syntax to anchor-client, 78% less code, no mock RPC needed.

[![Crates.io](https://img.shields.io/crates/v/anchor-litesvm.svg)](https://crates.io/crates/anchor-litesvm)
[![Documentation](https://docs.rs/anchor-litesvm/badge.svg)](https://docs.rs/anchor-litesvm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`anchor-litesvm` provides a streamlined testing experience for Anchor programs. It combines the familiar syntax of anchor-client with the speed of LiteSVM, plus comprehensive testing utilities.

**Key Benefits:**
- **78% less code** compared to raw LiteSVM
- **40% faster compilation** than anchor-client (no network dependencies)
- **No mock RPC** - zero configuration needed
- **Familiar syntax** - similar to anchor-client, transferable knowledge

## Installation

```toml
[dev-dependencies]
anchor-litesvm = "0.4"
```

## Quick Start

```rust
use anchor_litesvm::AnchorLiteSVM;
use litesvm_utils::{AssertionHelpers, TestHelpers};
use anchor_litesvm::Signer;

// 1. Generate `my_program::client::{accounts, args}` from the IDL...
anchor_lang::declare_program!(my_program);
// 2. ...and pair them with a caller-facing pubkey bundle per instruction,
//    generated from that same IDL.
anchor_litesvm::bundles_from_idl!(my_program);

#[test]
fn test_my_anchor_program() {
    // 3. One-line setup - no mock RPC needed. The name registers as a
    //    pubkey alias, so a failing send's printed logs read `my_program`
    //    instead of the raw program ID.
    let mut ctx = AnchorLiteSVM::build_with_program(
        my_program::ID,
        "my_program",
        include_bytes!("../target/deploy/my_program.so"),
    );

    // 4. Create accounts with built-in helpers
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 9).unwrap();

    // 5. Build the instruction from a bundle: only the accounts the IDL
    //    can't infer are fields here; PDAs and fixed addresses (the system
    //    program, well-known token programs) are derived/injected.
    let ix = ctx.program().build_ix(
        InitializeBundle {
            user: user.pubkey(),
            mint: mint.pubkey(),
        },
        my_program::client::args::Initialize { amount: 1_000_000 },
    );

    // 6. Execute and verify
    ctx.execute_instruction(ix, &[&user])
        .unwrap()
        .assert_success();

    // 7. Deserialize Anchor accounts
    let account_data: MyAccount = ctx.try_load(&pda).unwrap();
}
```

See [`crates/anchor-litesvm/examples/basic_usage.rs`](examples/basic_usage.rs) for
a fully compiling version of this flow against a small vault program's IDL.

## Why anchor-litesvm?

| Feature | anchor-client + LiteSVM | anchor-litesvm |
|---------|------------------------|----------------|
| Lines of Code | 279 lines | **106 lines** |
| Compilation | Slow (network deps) | **40% faster** |
| Setup | Mock RPC needed | **No config** |
| Token Operations | Manual (30+ lines) | **1 line** |
| Syntax | anchor-client | **Similar** |

### No More Account Ordering Bugs

The #1 pain point in Solana testing - eliminated:

```rust
// Raw LiteSVM - order matters, easy to get wrong
let instruction = Instruction {
    accounts: vec![
        AccountMeta::new(maker.pubkey(), true),   // Must be position 0
        AccountMeta::new(escrow_pda, false),       // Must be position 1
        // Wrong order = silent failure or wrong accounts
    ],
    ..
};

// anchor-litesvm - named fields, order doesn't matter
let ix = ctx.program()
    .accounts(my_program::client::accounts::Make {
        escrow: escrow_pda,  // Any order works
        maker: maker.pubkey(),
        // Compiler ensures all fields present
    })
    .args(...)
    .instruction()?;
```

## Features

### Bundled Instruction Construction

`bundles_from_idl!` reads the same IDL `declare_program!` does and, per
instruction, emits a `<Ix>Bundle` struct (one `Pubkey` field per account the
IDL can't infer), a `From<<Ix>Bundle> for <accounts struct>` that derives
every PDA and injects fixed addresses (the system program, well-known token
programs), and a `BuildableIx` pairing with the instruction's args type.
Adding an account to the program's IDL only requires regenerating; there's
no hand-written builder to keep in sync.

```rust
anchor_lang::declare_program!(my_program);
anchor_litesvm::bundles_from_idl!(my_program);

let ix = ctx.program().build_ix(
    MakeBundle { maker: maker.pubkey(), mint_a, mint_b },
    my_program::client::args::Make { amount, deposit },
);

ctx.execute_instruction(ix, &[&maker])?.assert_success();
```

### Manual Instruction Building

For a program without a shippable IDL, or a shape `bundles_from_idl!` can't
infer, build the accounts struct directly:

```rust
let ix = ctx.program()
    .accounts(my_program::client::accounts::Transfer {
        from: from_account,
        to: to_account,
        authority: user.pubkey(),
    })
    .args(my_program::client::args::Transfer { amount: 100 })
    .instruction()?;

ctx.execute_instruction(ix, &[&user])?.assert_success();
```

### Anchor Account Deserialization

```rust
// Deserialize with discriminator check
let account: MyAccount = ctx.try_load(&pda)?;

// Deserialize without check (for PDAs with custom layouts)
let account: MyAccount = ctx.try_load_unchecked(&pda)?;

// Test-oriented siblings that panic (with the address and cause) instead
// of returning a Result:
let account: MyAccount = ctx.load(&pda);
let account: MyAccount = ctx.load_unchecked(&pda);
```

### Event Parsing

```rust
use anchor_litesvm::EventHelpers;

let result = ctx.execute_instruction(ix, &[&user])?;

// Parse all events of a type
let events: Vec<TransferEvent> = result.parse_events()?;

// Parse first event
let event: TransferEvent = result.parse_event()?;

// Check if event was emitted
assert!(result.has_event::<TransferEvent>());
result.assert_event_emitted::<TransferEvent>();
```

### All litesvm-utils Features Included

Since `anchor-litesvm` builds on `litesvm-utils`, you get all utilities:

```rust
// Account creation
let user = ctx.svm.create_funded_account(10_000_000_000)?;

// Token operations
let mint = ctx.svm.create_token_mint(&user, 9)?;
let ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &user)?;
ctx.svm.mint_to(&mint.pubkey(), &ata, &user, 1_000_000)?;

// Assertions
ctx.svm.assert_token_balance(&ata, 1_000_000);
ctx.svm.assert_account_exists(&pda);

// PDA derivation
let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"seed"], &program_id);

// Transaction analysis
result.print_logs();
let cu = result.compute_units();
```

## Common Patterns

### Token Testing

```rust
let mint = ctx.svm.create_token_mint(&authority, 9)?;
let token_account = ctx.svm.create_associated_token_account(&mint.pubkey(), &owner)?;
ctx.svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000)?;

// After your instruction
ctx.svm.assert_token_balance(&token_account, expected_balance);
```

### PDA Usage

```rust
let (pda, bump) = ctx.svm.get_pda_with_bump(
    &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
    &my_program::ID,
);

let ix = ctx.program()
    .accounts(my_program::client::accounts::Initialize {
        escrow: pda,
        // ...
    })
    .args(my_program::client::args::Initialize { seed, bump })
    .instruction()?;
```

### Error Testing

```rust
let result = ctx.execute_instruction(ix, &[&user])?;

if !result.is_success() {
    result.print_logs();
    println!("Error: {:?}", result.error());
}

// Or assert specific errors: a substring match against logs or the error
// field, or an Anchor custom error code
result.assert_error("InsufficientFunds");
result.assert_error_code(6000);
```

## Testing

```bash
cargo test -p anchor-litesvm    # 11 tests
```

## Related Crates

- [`litesvm-utils`](https://crates.io/crates/litesvm-utils) - Framework-agnostic utilities (included)
- [`litesvm`](https://crates.io/crates/litesvm) - The underlying fast Solana VM
- [`anchor-lang`](https://crates.io/crates/anchor-lang) - Anchor framework

## License

MIT License - see [LICENSE](../../LICENSE) for details.
