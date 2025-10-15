# API Reference

Complete API documentation for `anchor-litesvm` and `litesvm-utils`.

## Table of Contents

- [Setup & Context](#setup--context)
- [Test Helpers](#test-helpers)
- [Instruction Building](#instruction-building)
- [Transaction Execution](#transaction-execution)
- [Assertions](#assertions)
- [Error Testing](#error-testing)
- [Event Parsing](#event-parsing)
- [Account Operations](#account-operations)
- [PDA Operations](#pda-operations)
- [Clock & Slot](#clock--slot)

---

## Setup & Context

### `AnchorLiteSVM::build_with_program()`

Create a new test context with a single program.

```rust
pub fn build_with_program(program_id: Pubkey, program_bytes: &[u8]) -> AnchorContext
```

**Parameters:**
- `program_id`: The program ID to deploy
- `program_bytes`: The compiled program bytes (`.so` file)

**Returns:** `AnchorContext` ready for testing

**Example:**
```rust
let mut ctx = AnchorLiteSVM::build_with_program(
    my_program::ID,
    include_bytes!("../target/deploy/my_program.so"),
);
```

---

### `AnchorLiteSVM::new()`

Create a new builder for configuring the test environment.

```rust
pub fn new() -> Self
```

**Returns:** Builder for chaining configuration

**Example:**
```rust
let mut ctx = AnchorLiteSVM::new()
    .deploy_program(program_id1, program_bytes1)
    .deploy_program(program_id2, program_bytes2)
    .build();
```

---

### `AnchorLiteSVM::deploy_program()`

Add a program to the builder.

```rust
pub fn deploy_program(self, program_id: Pubkey, program_bytes: &[u8]) -> Self
```

**Parameters:**
- `program_id`: Program ID
- `program_bytes`: Compiled program

**Returns:** Self for chaining

**Example:**
```rust
let ctx = AnchorLiteSVM::new()
    .deploy_program(program_id, program_bytes)
    .build();
```

---

### `AnchorLiteSVM::build_with_programs()`

Create context with multiple programs.

```rust
pub fn build_with_programs(programs: &[(Pubkey, &[u8])]) -> AnchorContext
```

**Parameters:**
- `programs`: Slice of (program_id, program_bytes) tuples

**Returns:** `AnchorContext` with all programs deployed

**Example:**
```rust
let programs = vec![
    (program_id1, program_bytes1),
    (program_id2, program_bytes2),
];
let mut ctx = AnchorLiteSVM::build_with_programs(&programs);
```

---

## Test Helpers

All helper methods are accessed via `ctx.svm` and use the `TestHelpers` trait.

### `create_funded_account()`

Create a new keypair with SOL airdropped.

```rust
fn create_funded_account(&mut self, lamports: u64) -> Result<Keypair, Box<dyn Error>>
```

**Parameters:**
- `lamports`: Amount of SOL to fund (in lamports)

**Returns:** Funded `Keypair`

**Example:**
```rust
let user = ctx.svm.create_funded_account(10_000_000_000)?; // 10 SOL
```

---

### `create_funded_accounts()`

Create multiple funded accounts at once.

```rust
fn create_funded_accounts(&mut self, count: usize, lamports: u64)
    -> Result<Vec<Keypair>, Box<dyn Error>>
```

**Parameters:**
- `count`: Number of accounts to create
- `lamports`: Amount per account

**Returns:** Vector of funded `Keypair`s

**Example:**
```rust
let accounts = ctx.svm.create_funded_accounts(5, 1_000_000_000)?;
```

---

### `create_token_mint()`

Create and initialize a token mint.

```rust
fn create_token_mint(&mut self, authority: &Keypair, decimals: u8)
    -> Result<Keypair, Box<dyn Error>>
```

**Parameters:**
- `authority`: Mint authority keypair
- `decimals`: Number of decimals (usually 9)

**Returns:** Mint `Keypair`

**Example:**
```rust
let mint = ctx.svm.create_token_mint(&authority, 9)?;
```

---

### `create_token_account()`

Create a token account (non-ATA).

```rust
fn create_token_account(&mut self, mint: &Pubkey, owner: &Keypair)
    -> Result<Keypair, Box<dyn Error>>
```

**Parameters:**
- `mint`: Token mint pubkey
- `owner`: Token account owner

**Returns:** Token account `Keypair`

**Example:**
```rust
let token_account = ctx.svm.create_token_account(&mint.pubkey(), &owner)?;
```

---

### `create_associated_token_account()`

Create an associated token account.

```rust
fn create_associated_token_account(&mut self, mint: &Pubkey, owner: &Keypair)
    -> Result<Pubkey, Box<dyn Error>>
```

**Parameters:**
- `mint`: Token mint pubkey
- `owner`: ATA owner

**Returns:** ATA `Pubkey`

**Example:**
```rust
let ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &owner)?;
```

---

### `mint_to()`

Mint tokens to an account.

```rust
fn mint_to(&mut self, mint: &Pubkey, account: &Pubkey,
           authority: &Keypair, amount: u64)
    -> Result<(), Box<dyn Error>>
```

**Parameters:**
- `mint`: Mint pubkey
- `account`: Destination token account
- `authority`: Mint authority
- `amount`: Amount to mint

**Example:**
```rust
ctx.svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000)?;
```

---

## Instruction Building

### `ctx.program()`

Get the program instance for building instructions.

```rust
pub fn program(&self) -> Program
```

**Returns:** `Program` instance

**Example:**
```rust
let ix = ctx.program()
    .accounts(...)
    .args(...)
    .instruction()?;
```

---

### `Program::accounts()`

Start building an instruction with accounts (simplified syntax similar to anchor-client).

```rust
pub fn accounts<T: ToAccountMetas>(self, accounts: T) -> InstructionBuilder
```

**Parameters:**
- `accounts`: Struct implementing `ToAccountMetas` (generated by `declare_program`)

**Returns:** `InstructionBuilder`

**Example:**
```rust
let builder = ctx.program().accounts(my_program::client::accounts::Initialize { ... });
```

---

### `InstructionBuilder::args()`

Set the instruction arguments.

```rust
pub fn args<T: InstructionData>(self, args: T) -> Self
```

**Parameters:**
- `args`: Struct implementing `InstructionData` (generated by `declare_program`)

**Returns:** Self for chaining

**Example:**
```rust
builder.args(my_program::client::args::Initialize {
    amount: 1_000_000,
})
```

---

### `InstructionBuilder::instruction()`

Build and return the instruction (recommended method).

```rust
pub fn instruction(self) -> Result<Instruction, Box<dyn std::error::Error>>
```

**Returns:** Single `Instruction`

**Example:**
```rust
let ix = builder.instruction()?;
```

---

## Transaction Execution

### `ctx.execute_instruction()`

Execute a single instruction.

```rust
pub fn execute_instruction(&mut self, instruction: Instruction,
                          signers: &[&Keypair])
    -> Result<TransactionResult, Box<dyn std::error::Error>>
```

**Parameters:**
- `instruction`: Instruction to execute
- `signers`: Array of signer keypairs

**Returns:** `TransactionResult` for analysis

**Example:**
```rust
let result = ctx.execute_instruction(ix, &[&user])?;
result.assert_success();
```

---

### `ctx.execute_instructions()`

Execute multiple instructions in one transaction.

```rust
pub fn execute_instructions(&mut self,
                           instructions: Vec<Instruction>,
                           signers: &[&Keypair])
    -> Result<TransactionResult, Box<dyn std::error::Error>>
```

**Parameters:**
- `instructions`: Vector of instructions
- `signers`: Array of signers

**Returns:** `TransactionResult`

**Example:**
```rust
let result = ctx.execute_instructions(vec![ix1, ix2], &[&user])?;
```

---

### `TransactionResult::assert_success()`

Assert that transaction succeeded.

```rust
pub fn assert_success(&self) -> &Self
```

**Panics:** If transaction failed

**Example:**
```rust
result.assert_success();
```

---

### `TransactionResult::is_success()`

Check if transaction succeeded.

```rust
pub fn is_success(&self) -> bool
```

**Returns:** `true` if successful

**Example:**
```rust
if result.is_success() {
    println!("Success!");
}
```

---

### `TransactionResult::compute_units()`

Get compute units consumed.

```rust
pub fn compute_units(&self) -> u64
```

**Returns:** Compute units used

**Example:**
```rust
let cu = result.compute_units();
assert!(cu < 200_000, "Too many compute units");
```

---

### `TransactionResult::logs()`

Get all transaction logs.

```rust
pub fn logs(&self) -> &[String]
```

**Returns:** Slice of log messages

**Example:**
```rust
for log in result.logs() {
    println!("{}", log);
}
```

---

### `TransactionResult::has_log()`

Check if logs contain a message.

```rust
pub fn has_log(&self, message: &str) -> bool
```

**Parameters:**
- `message`: Message to search for

**Returns:** `true` if found

**Example:**
```rust
assert!(result.has_log("Transfer complete"));
```

---

### `TransactionResult::find_log()`

Find first log containing pattern.

```rust
pub fn find_log(&self, pattern: &str) -> Option<&String>
```

**Parameters:**
- `pattern`: Pattern to search for

**Returns:** First matching log or `None`

**Example:**
```rust
if let Some(log) = result.find_log("Result:") {
    println!("Found: {}", log);
}
```

---

### `TransactionResult::print_logs()`

Pretty-print all logs for debugging.

```rust
pub fn print_logs(&self)
```

**Example:**
```rust
result.print_logs();
```

---

## Assertions

All assertion methods are accessed via `ctx.svm` and use the `AssertionHelpers` trait.

### `assert_account_exists()`

Assert that an account exists.

```rust
fn assert_account_exists(&self, pubkey: &Pubkey)
```

**Parameters:**
- `pubkey`: Account to check

**Panics:** If account doesn't exist

**Example:**
```rust
ctx.svm.assert_account_exists(&user.pubkey());
```

---

### `assert_account_closed()`

Assert that an account is closed.

```rust
fn assert_account_closed(&self, pubkey: &Pubkey)
```

**Parameters:**
- `pubkey`: Account to check

**Panics:** If account still exists with data/lamports

**Example:**
```rust
ctx.svm.assert_account_closed(&temp_account);
```

---

### `assert_token_balance()`

Assert token account balance.

```rust
fn assert_token_balance(&self, token_account: &Pubkey, expected: u64)
```

**Parameters:**
- `token_account`: Token account pubkey
- `expected`: Expected balance

**Panics:** If balance doesn't match

**Example:**
```rust
ctx.svm.assert_token_balance(&token_account, 1_000_000);
```

---

### `assert_sol_balance()`

Assert SOL balance.

```rust
fn assert_sol_balance(&self, pubkey: &Pubkey, expected: u64)
```

**Parameters:**
- `pubkey`: Account pubkey
- `expected`: Expected lamports

**Panics:** If balance doesn't match

**Example:**
```rust
ctx.svm.assert_sol_balance(&user.pubkey(), 10_000_000_000);
```

---

### `assert_mint_supply()`

Assert token mint supply.

```rust
fn assert_mint_supply(&self, mint: &Pubkey, expected: u64)
```

**Parameters:**
- `mint`: Mint pubkey
- `expected`: Expected supply

**Panics:** If supply doesn't match

**Example:**
```rust
ctx.svm.assert_mint_supply(&mint.pubkey(), 1_000_000);
```

---

### `assert_account_owner()`

Assert account owner.

```rust
fn assert_account_owner(&self, account: &Pubkey, expected_owner: &Pubkey)
```

**Parameters:**
- `account`: Account to check
- `expected_owner`: Expected owner program

**Panics:** If owner doesn't match

**Example:**
```rust
ctx.svm.assert_account_owner(&token_account, &spl_token::id());
```

---

### `assert_account_data_len()`

Assert account data length.

```rust
fn assert_account_data_len(&self, account: &Pubkey, expected_len: usize)
```

**Parameters:**
- `account`: Account to check
- `expected_len`: Expected data length

**Panics:** If length doesn't match

**Example:**
```rust
ctx.svm.assert_account_data_len(&token_account, 165);
```

---

## Error Testing

New error assertion methods on `TransactionResult`.

### `assert_failure()`

Assert that transaction failed.

```rust
pub fn assert_failure(&self) -> &Self
```

**Panics:** If transaction succeeded

**Example:**
```rust
result.assert_failure();
```

---

### `assert_error()`

Assert transaction failed with specific error message.

```rust
pub fn assert_error(&self, expected_error: &str) -> &Self
```

**Parameters:**
- `expected_error`: Expected error substring

**Panics:** If transaction succeeded or error doesn't match

**Example:**
```rust
result.assert_error("insufficient funds");
```

---

### `assert_error_code()`

Assert transaction failed with specific Anchor error code.

```rust
pub fn assert_error_code(&self, error_code: u32) -> &Self
```

**Parameters:**
- `error_code`: Expected Anchor error code (e.g., 6000)

**Panics:** If transaction succeeded or code doesn't match

**Example:**
```rust
result.assert_error_code(6000); // Custom error code
```

---

### `assert_anchor_error()`

Assert transaction failed with named Anchor error.

```rust
pub fn assert_anchor_error(&self, error_name: &str) -> &Self
```

**Parameters:**
- `error_name`: Expected Anchor error name

**Panics:** If transaction succeeded or error not found

**Example:**
```rust
result.assert_anchor_error("InsufficientFunds");
```

---

### `assert_log_error()`

Assert error message appears in logs.

```rust
pub fn assert_log_error(&self, error_message: &str) -> &Self
```

**Parameters:**
- `error_message`: Expected error message

**Panics:** If message not found in logs

**Example:**
```rust
result.assert_log_error("Transfer amount exceeds balance");
```

---

## Event Parsing

Event parsing methods on `TransactionResult` via `EventHelpers` trait.

### `parse_events()`

Parse all events of a specific type.

```rust
fn parse_events<T>(&self) -> Result<Vec<T>, EventError>
where
    T: AnchorDeserialize + Discriminator + Event
```

**Type Parameter:**
- `T`: Event type to parse

**Returns:** Vector of parsed events

**Example:**
```rust
let events: Vec<TransferEvent> = result.parse_events()?;
for event in events {
    println!("Transfer: {} tokens", event.amount);
}
```

---

### `parse_event()`

Parse first event of a specific type.

```rust
fn parse_event<T>(&self) -> Result<T, EventError>
where
    T: AnchorDeserialize + Discriminator + Event
```

**Type Parameter:**
- `T`: Event type to parse

**Returns:** First event of type `T`

**Example:**
```rust
let event: TransferEvent = result.parse_event()?;
assert_eq!(event.amount, 1_000_000);
```

---

### `assert_event_emitted()`

Assert at least one event was emitted.

```rust
fn assert_event_emitted<T>(&self)
where
    T: AnchorDeserialize + Discriminator + Event
```

**Type Parameter:**
- `T`: Event type

**Panics:** If no events of type `T` found

**Example:**
```rust
result.assert_event_emitted::<TransferEvent>();
```

---

### `assert_event_count()`

Assert specific number of events.

```rust
fn assert_event_count<T>(&self, expected_count: usize)
where
    T: AnchorDeserialize + Discriminator + Event
```

**Parameters:**
- `expected_count`: Expected number of events

**Panics:** If count doesn't match

**Example:**
```rust
result.assert_event_count::<TransferEvent>(2);
```

---

### `has_event()`

Check if event was emitted.

```rust
fn has_event<T>(&self) -> bool
where
    T: AnchorDeserialize + Discriminator + Event
```

**Returns:** `true` if event found

**Example:**
```rust
if result.has_event::<TransferEvent>() {
    println!("Transfer occurred");
}
```

---

## Account Operations

### `ctx.get_account()`

Get and deserialize an Anchor account.

```rust
pub fn get_account<T>(&self, address: &Pubkey) -> Result<T, AccountError>
where
    T: AccountDeserialize
```

**Type Parameter:**
- `T`: Account type

**Returns:** Deserialized account

**Example:**
```rust
let account: MyAccountType = ctx.get_account(&pda)?;
assert_eq!(account.authority, user.pubkey());
```

---

### `ctx.get_account_unchecked()`

Get account without discriminator check.

```rust
pub fn get_account_unchecked<T>(&self, address: &Pubkey) -> Result<T, AccountError>
where
    T: AccountDeserialize
```

**Type Parameter:**
- `T`: Account type

**Returns:** Deserialized account

**Example:**
```rust
let account: MyAccountType = ctx.get_account_unchecked(&pda)?;
```

---

### `ctx.account_exists()`

Check if account exists.

```rust
pub fn account_exists(&self, pubkey: &Pubkey) -> bool
```

**Parameters:**
- `pubkey`: Account to check

**Returns:** `true` if exists

**Example:**
```rust
if ctx.account_exists(&pda) {
    println!("Account exists");
}
```

---

## PDA Operations

### `get_pda()`

Calculate a PDA (just the address).

```rust
fn get_pda(&self, seeds: &[&[u8]], program_id: &Pubkey) -> Pubkey
```

**Parameters:**
- `seeds`: Array of seed slices
- `program_id`: Program ID

**Returns:** PDA `Pubkey`

**Example:**
```rust
let pda = ctx.svm.get_pda(
    &[b"vault", user.pubkey().as_ref()],
    &program_id
);
```

---

### `get_pda_with_bump()`

Calculate a PDA with bump seed.

```rust
fn get_pda_with_bump(&self, seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8)
```

**Parameters:**
- `seeds`: Array of seed slices
- `program_id`: Program ID

**Returns:** Tuple of (PDA, bump)

**Example:**
```rust
let (pda, bump) = ctx.svm.get_pda_with_bump(
    &[b"vault", user.pubkey().as_ref()],
    &program_id
);
```

---

### `derive_pda()`

Alias for `get_pda_with_bump()`.

```rust
fn derive_pda(&self, seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8)
```

**Example:**
```rust
let (pda, bump) = ctx.svm.derive_pda(&[b"seed"], &program_id);
```

---

## Clock & Slot

### `get_current_slot()`

Get the current slot number.

```rust
fn get_current_slot(&self) -> u64
```

**Returns:** Current slot

**Example:**
```rust
let slot = ctx.svm.get_current_slot();
println!("Current slot: {}", slot);
```

---

### `advance_slot()`

Advance the slot by a specific amount.

```rust
fn advance_slot(&mut self, slots: u64)
```

**Parameters:**
- `slots`: Number of slots to advance

**Example:**
```rust
ctx.svm.advance_slot(100);
let new_slot = ctx.svm.get_current_slot();
```

---

## Additional Utilities

### `ctx.airdrop()`

Airdrop lamports to an account.

```rust
pub fn airdrop(&mut self, pubkey: &Pubkey, lamports: u64)
    -> Result<(), Box<dyn std::error::Error>>
```

**Parameters:**
- `pubkey`: Recipient account
- `lamports`: Amount to airdrop

**Example:**
```rust
ctx.airdrop(&user.pubkey(), 1_000_000_000)?;
```

---

### `ctx.latest_blockhash()`

Get the latest blockhash.

```rust
pub fn latest_blockhash(&self) -> Hash
```

**Returns:** Latest blockhash

**Example:**
```rust
let blockhash = ctx.latest_blockhash();
```

---

### `ctx.payer()`

Get the payer keypair.

```rust
pub fn payer(&self) -> &Keypair
```

**Returns:** Reference to payer `Keypair`

**Example:**
```rust
let payer = ctx.payer();
println!("Payer: {}", payer.pubkey());
```

---

## Complete Example

Here's a complete example using many of these APIs:

```rust
use anchor_litesvm::{AnchorLiteSVM, TestHelpers, AssertionHelpers, EventHelpers};
use solana_sdk::signature::Signer;

anchor_lang::declare_program!(my_program);

#[test]
fn test_complete_workflow() {
    // Setup
    let mut ctx = AnchorLiteSVM::build_with_program(
        my_program::ID,
        include_bytes!("../target/deploy/my_program.so"),
    );

    // Create accounts
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 9).unwrap();
    let token_account = ctx.svm
        .create_associated_token_account(&mint.pubkey(), &user)
        .unwrap();

    // Mint tokens
    ctx.svm.mint_to(&mint.pubkey(), &token_account, &user, 1_000_000).unwrap();

    // Calculate PDA
    let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"vault"], &my_program::ID);

    // Build instruction
    let ix = ctx.program()
        .accounts(my_program::client::accounts::Initialize {
            user: user.pubkey(),
            vault: pda,
            token_account,
            system_program: system_program::id(),
        })
        .args(my_program::client::args::Initialize {
            bump,
            amount: 500_000,
        })
        .instruction()
        .unwrap();

    // Execute
    let result = ctx.execute_instruction(ix, &[&user]).unwrap();

    // Assert success
    result.assert_success();

    // Check compute units
    assert!(result.compute_units() < 200_000);

    // Check logs
    assert!(result.has_log("Initialization complete"));

    // Check events
    result.assert_event_emitted::<InitializeEvent>();
    let event: InitializeEvent = result.parse_event().unwrap();
    assert_eq!(event.amount, 500_000);

    // Verify account states
    ctx.svm.assert_account_exists(&pda);
    ctx.svm.assert_token_balance(&token_account, 500_000);
    ctx.svm.assert_account_owner(&pda, &my_program::ID);

    // Read account data
    let account: VaultAccount = ctx.get_account(&pda).unwrap();
    assert_eq!(account.authority, user.pubkey());
    assert_eq!(account.bump, bump);
}
```

---

## See Also

- [Quick Start Guide](QUICK_START.md) - Get started in 5 minutes
- [Migration Guide](MIGRATION.md) - Migrate from raw LiteSVM
- [Examples](../examples/) - Runnable code examples
- [GitHub Repository](https://github.com/brimigs/anchor-litesvm)
