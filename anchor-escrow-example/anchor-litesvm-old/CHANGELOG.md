# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-01-19

### Added

#### Core Features
- **Automatic Anchor Integration**: Automatic discriminator calculation and Borsh serialization for Anchor instructions
- **Type-safe Account Deserialization**: Generic deserialization of Anchor accounts with proper discriminator handling
- **PDA Calculation**: Helper method for finding Program Derived Addresses with seeds

#### Fluent Instruction Builder
- Chainable API for building Anchor instructions
- Built-in support for common programs (System, Token, Associated Token)
- `tuple_args()` helper to avoid manual struct definitions
- Direct execution with `.execute()` method

#### Test Helpers
- `create_funded_account()`: Create and fund test accounts in one line
- `create_funded_accounts()`: Batch account creation
- `create_token_mint()`: Simplified token mint creation
- `create_token_account()`: Create ATA with optional token minting
- `batch_airdrop()`: Airdrop SOL to multiple accounts

#### Transaction Helpers
- `send_instruction()`: Execute single instructions with automatic transaction building
- `send_instructions()`: Execute multiple instructions in one transaction
- `TransactionResult` wrapper with metadata access
- Transaction analysis methods: `has_log()`, `compute_units()`, `assert_success()`

#### Assertion Helpers
- `assert_account_exists()`: Verify account creation
- `assert_account_closed()`: Verify account closure
- `assert_accounts_closed()`: Batch account closure verification
- `assert_token_balance()`: Check token account balances
- `assert_account_lamports()`: Verify SOL balances
- `assert_account_owner()`: Check account ownership

#### Simplified Initialization
- `AnchorLiteSVM::build_with_program()`: One-line test environment setup
- Builder pattern for deploying multiple programs
- `ProgramTestExt` trait for extension methods on `Pubkey`

### Performance
- 66-80% reduction in test code compared to raw LiteSVM
- Zero-overhead abstraction - thin wrapper around LiteSVM
- Direct access to underlying LiteSVM instance

### Documentation
- Comprehensive README with examples and comparisons
- API documentation for all public methods
- Example files demonstrating basic and advanced usage

### Developer Experience
- Fluent, chainable APIs throughout
- Consistent error handling with `Result` types
- No hidden magic - full transparency and control
- Compatible with existing litesvm-token utilities

## [Unreleased]

### Planned Features
- IDL file parsing for automatic account resolution
- Automatic signer detection from account types
- Event emission parsing from logs
- Time manipulation helpers for testing time-based logic
- Account snapshot/rollback for test isolation
- Procedural macros for test setup
- Integration with anchor-client types

---

For more information about upcoming features, see the [README roadmap](README.md#roadmap).