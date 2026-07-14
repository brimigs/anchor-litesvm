//! # anchor-litesvm
//!
//! Testing framework for Anchor programs using LiteSVM.
//!
//! This crate provides a **simplified syntax similar to anchor-client** but without RPC overhead,
//! achieving **78% code reduction** compared to raw LiteSVM.
//!
//! ## Why anchor-litesvm?
//!
//! | Feature | anchor-client + LiteSVM | anchor-litesvm |
//! |---------|-------------------------|----------------|
//! | **Code Lines** | 279 | **106 (78% less)** |
//! | **Compilation** | Slow (network deps) | **40% faster** |
//! | **Setup** | Mock RPC needed | **One line** |
//! | **Syntax** | anchor-client | **Similar to anchor-client** |
//! | **Helpers** | Manual | **Built-in** |
//!
//! ## Key Features
//!
//! - **Simplified Syntax**: Similar to anchor-client
//! - **No Mock RPC Setup**: One-line initialization
//! - **Integrated Test Helpers**: Token operations, assertions, event parsing
//! - **Familiar API**: If you know anchor-client, you know this
//! - **Transferable Knowledge**: Test skills apply to production
//! - **Type Safety**: Compile-time validation with Anchor types
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use anchor_litesvm::{AnchorLiteSVM, TestHelpers, AssertionHelpers, Signer};
//!
//! // 1. Generate client types from your program
//! anchor_lang::declare_program!(my_program);
//!
//! #[test]
//! fn test_my_program() {
//!     // 2. One-line setup (no mock RPC needed). The name registers as
//!     //    a pubkey alias so a failing send's printed logs read
//!     //    `my_program` instead of the raw program ID.
//!     let mut ctx = AnchorLiteSVM::build_with_program(
//!         my_program::ID,
//!         "my_program",
//!         include_bytes!("../target/deploy/my_program.so"),
//!     );
//!
//!     // 3. Create test accounts with helpers
//!     let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
//!     let mint = ctx.svm.create_token_mint(&user, 9).unwrap();
//!
//!     // 4. Build instruction (simplified syntax - similar to anchor client)
//!     let ix = ctx.program()
//!         .accounts(my_program::client::accounts::Transfer {
//!             from: sender_account,
//!             to: recipient_account,
//!             authority: user.pubkey(),
//!             token_program: spl_token::id(),
//!         })
//!         .args(my_program::client::args::Transfer { amount: 100 })
//!         .instruction()?;
//!
//!     // 5. Execute and verify
//!     ctx.execute_instruction(ix, &[&user])?.assert_success();
//!     ctx.svm.assert_token_balance(&recipient_account, 100);
//! }
//! ```
//!
//! ## Common Patterns
//!
//! ### Token Operations
//!
//! ```rust,ignore
//! use litesvm_utils::TestHelpers;
//!
//! let mint = ctx.svm.create_token_mint(&authority, 9)?;
//! let token_account = ctx.svm.create_associated_token_account(&mint.pubkey(), &owner)?;
//! ctx.svm.mint_to(&mint.pubkey(), &token_account, &authority, 1_000_000)?;
//!
//! // Read SPL Token balance. `None` means the account doesn't exist
//! // (so closed-vault assertions read tightly).
//! assert_eq!(ctx.svm.token_balance(&token_account), Some(1_000_000));
//! assert!(ctx.svm.token_balance(&closed_vault).is_none());
//! ```
//!
//! ### PDA Derivation
//!
//! ```rust,ignore
//! // Just the address
//! let pda = ctx.svm.get_pda(&[b"vault", user.pubkey().as_ref()], &program_id);
//!
//! // With bump seed
//! let (pda, bump) = ctx.svm.get_pda_with_bump(&[b"vault"], &program_id);
//! ```
//!
//! ### Error Testing
//!
//! ```rust,ignore
//! let result = ctx.execute_instruction(ix, &[&user])?;
//! // Pick the assertion that matches your scenario; each consumes `result`.
//! result.assert_failure();                          // generic
//! // or: result.assert_error("EscrowExpired");     // substring in logs or error field
//! // or: result.assert_error_code(6000);           // Anchor custom error code
//! ```
//!
//! ### Event Parsing
//!
//! ```rust,ignore
//! use anchor_litesvm::EventHelpers;
//!
//! let events: Vec<TransferEvent> = result.parse_events()?;
//! result.assert_event_emitted::<TransferEvent>();
//! ```
//!
//! ### Account Deserialization
//!
//! ```rust,ignore
//! let account: MyAccountType = ctx.try_load(&pda)?;
//! assert_eq!(account.authority, user.pubkey());
//! ```
//!
//! ### Bundled Instruction Construction
//!
//! Instead of hand-filling `accounts::Foo { .. }.to_account_metas(None)`
//! plus `instruction::Foo { .. }.data()` per instruction, generate a bundle
//! type per instruction straight from the program's IDL and let
//! [`Program::build_ix`](program::Program::build_ix) do the wiring:
//!
//! ```rust,ignore
//! anchor_lang::declare_program!(my_program);
//! anchor_litesvm::bundles_from_idl!(my_program);
//!
//! let ix = ctx.program().build_ix(
//!     MakeBundle { maker: maker.pubkey(), mint_a, vault, /* ... */ },
//!     my_program::client::args::Make { amount, deposit, .. },
//! );
//! ```
//!
//! `bundles_from_idl!` emits one `<Ix>Bundle` struct per instruction (one
//! `Pubkey` field per account the IDL lists), a `From<<Ix>Bundle> for
//! <accounts struct>` (auto-injecting accounts it can infer, like the system
//! program), and a [`BuildableIx`] impl pairing the
//! bundle with its args type. Adding an account to the program's IDL only
//! requires regenerating; there's no hand-written builder to keep in sync.
//!
//! ### Send + Assert Shortcuts
//!
//! Most tests end in one of two shapes; the shortcuts collapse the
//! send + unwrap + assert chain into a single call.
//!
//! Use [`AnchorContext`]'s send helpers when the context owns the
//! alias table (`ctx.alias(pk, "name")` builds it up); use the bare
//! [`TransactionHelpers`] variants on `ctx.svm` when threading an
//! external alias table directly.
//!
//! ```rust,ignore
//! // Context-owned aliases: no per-call `&Aliases`. The returned
//! // TransactionResult carries the alias table, so a chained
//! // `.print_logs()` reads it implicitly.
//! ctx.alias(maker.pubkey(), "maker");
//! ctx.send_ok(ix, &[&maker]).print_logs();
//!
//! // Expected failure: substring match against logs + the error field,
//! // same semantics as TransactionResult::assert_error.
//! ctx.send_err_named(ix, &[&taker], "EscrowExpired");
//!
//! // Bare LiteSVM variant: external alias table threaded per call.
//! ctx.svm.send_ok(ix, &[&maker], &aliases).print_logs();
//! ```
//!
//! ## Documentation
//!
//! - [Quick Start Guide](https://github.com/brimigs/anchor-litesvm/blob/main/docs/QUICK_START.md)
//! - [API Reference](https://github.com/brimigs/anchor-litesvm/blob/main/docs/API_REFERENCE.md)
//! - [Migration Guide](https://github.com/brimigs/anchor-litesvm/blob/main/docs/MIGRATION.md)
//! - [Examples](https://github.com/brimigs/anchor-litesvm/tree/main/examples)
//!
//! ## Modules
//!
//! - [`account`] - Account deserialization utilities
//! - [`buildable`] - The `BuildableIx` bundle/args pairing
//! - [`builder`] - Test environment builders
//! - [`context`] - Main test context (`AnchorContext`)
//! - [`events`] - Event parsing helpers
//! - [`instruction`] - Instruction building utilities
//! - [`program`] - Simplified Program API
//! - [`tx`] - Fluent build + send + expect chain

pub mod account;
pub mod buildable;
pub mod builder;
pub mod context;
mod event_idl;
pub mod events;
pub mod idl_sanitize;
pub mod instruction;
pub mod program;
pub mod tx;

// Re-export main types for convenience
pub use account::{get_anchor_account, get_anchor_account_unchecked, AccountError};
pub use anchor_litesvm_derive::bundles_from_idl;
pub use buildable::BuildableIx;
pub use builder::{AnchorLiteSVM, ProgramTestExt};
pub use context::AnchorContext;
pub use events::{parse_event_data, EventError, EventHelpers};
pub use idl_sanitize::sanitize_idl;
pub use instruction::{build_anchor_instruction, calculate_anchor_discriminator};
pub use program::{InstructionBuilder, Program};
pub use tx::Resolvable;
pub use tx::Tx;

// Re-export litesvm-utils functionality for convenience
pub use litesvm_utils::{
    metaplex::{
        Creator, MetadataArgs, MetaplexHelpers, TokenStandard, METADATA_SEED, MPL_TOKEN_METADATA_ID,
    },
    naming::{deterministic_keypair, seed_bytes, ActorRegistry},
    token_hooks::TransferHookTesting,
    tokens::{TokenFabrication, TokenProgram, TOKEN_2022_ID},
    Aliases, AssertionHelpers, LiteSVMBuilder, TestHelpers, TransactionError, TransactionHelpers,
    TransactionResult,
};

// Re-export commonly used external types
pub use anchor_lang::{AccountDeserialize, AnchorSerialize};
pub use anchor_litesvm_compat::{Keypair, LiteSVM, Signer, TransactionMetadata};
pub use solana_program::instruction::{AccountMeta, Instruction};
pub use solana_program::pubkey::Pubkey;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use borsh::BorshSerialize;
    use litesvm_utils::TestHelpers;
    use solana_system_interface::instruction as system_instruction;

    /// `ctx.alias` extends the context-owned table, and `ctx.send_ok`
    /// returns a result that carries those aliases so a no-arg
    /// `logs_string()` resolves them.
    #[test]
    fn anchor_context_send_ok_threads_self_aliases_through_to_result() {
        let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
        let payer = ctx.svm.create_funded_account(1_000_000_000).unwrap();
        let recipient = Keypair::new();

        // Override the well-known System alias so the result's rendered
        // header line proves *this* context's table (not just the built-in
        // default) rode along on the send.
        ctx.alias(solana_system_interface::program::ID, "SystemFromCtx");

        let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
        let out = ctx.send_ok(ix, &[&payer]).logs_string();

        assert!(
            out.contains("Program: SystemFromCtx"),
            "ctx.alias should flow through ctx.send_ok to the no-arg logs_string(); got:\n{out}"
        );
    }

    #[test]
    fn cast_mint_is_deterministic_aliased_and_usable() {
        use litesvm_utils::naming::deterministic_keypair;

        let program_id = Pubkey::new_unique();
        let mut ctx = AnchorContext::new(LiteSVM::new(), program_id);

        let authority = ctx.cast_actor("Authority");
        let mint = ctx.cast_mint("USDC", &authority, 6);

        // Deterministic: derived from (program_id, name), the same domain
        // cast_actor uses, so the address is stable across runs.
        assert_eq!(
            mint,
            deterministic_keypair(&program_id.to_string(), "USDC").pubkey()
        );
        // Aliased under its cast name.
        assert_eq!(ctx.label(&mint), "USDC");
        // Real and usable: an ATA can be created against it and funded.
        let ata = ctx
            .svm
            .create_associated_token_account(&mint, &authority)
            .unwrap();
        ctx.svm.mint_to(&mint, &ata, &authority, 1_000_000).unwrap();
        assert_eq!(ctx.svm.token_balance(&ata), Some(1_000_000));
    }

    #[test]
    fn pda_derives_against_this_programs_id() {
        use litesvm_utils::TestHelpers;

        let program_id = Pubkey::new_unique();
        let ctx = AnchorContext::new(LiteSVM::new(), program_id);
        let seeds: &[&[u8]] = &[b"counter", &[1, 2, 3]];

        // `ctx.pda` supplies this program's id; it matches the explicit form.
        let (addr, bump) = Pubkey::find_program_address(seeds, &program_id);
        assert_eq!(ctx.pda(seeds), addr);
        assert_eq!(ctx.pda_with_bump(seeds), (addr, bump));
        // ...and agrees with the generic `get_pda` given the same id.
        assert_eq!(ctx.pda(seeds), ctx.svm.get_pda(seeds, &program_id));
    }

    #[test]
    fn fund_ata_creates_aliased_funded_holding() {
        let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
        let issuer = ctx.cast_actor("Issuer");
        let alice = ctx.cast_actor("Alice");
        let usdc = ctx.cast_mint("USDC", &issuer, 6);

        let alice_usdc = ctx.fund_ata(&alice, &usdc, &issuer, 1_000_000);
        // Funded with the requested amount...
        assert_eq!(ctx.svm.token_balance(&alice_usdc), Some(1_000_000));
        // ...and aliased under the composed owner/mint name.
        assert_eq!(ctx.label(&alice_usdc), "Alice/USDC");

        // amount == 0 still leaves a real, empty, aliased account.
        let bob = ctx.cast_actor("Bob");
        let bob_usdc = ctx.fund_ata(&bob, &usdc, &issuer, 0);
        assert_eq!(ctx.svm.token_balance(&bob_usdc), Some(0));
        assert_eq!(ctx.label(&bob_usdc), "Bob/USDC");
    }

    #[test]
    fn cast_actor_with_sol_funds_exact_and_aliases() {
        use litesvm_utils::naming::deterministic_keypair;

        let program_id = Pubkey::new_unique();
        let mut ctx = AnchorContext::new(LiteSVM::new(), program_id);

        let whale = ctx.cast_actor_with_sol("Whale", 5_000_000_000);
        // Exact stake, not the 100 SOL float.
        assert_eq!(ctx.svm.get_balance(&whale.pubkey()), Some(5_000_000_000));
        // Same deterministic derivation and alias as cast_actor.
        assert_eq!(
            whale.pubkey(),
            deterministic_keypair(&program_id.to_string(), "Whale").pubkey()
        );
        assert_eq!(ctx.label(&whale.pubkey()), "Whale");
    }

    #[test]
    #[should_panic(expected = "already used in this scenario")]
    fn cast_vocabulary_rejects_a_duplicate_name() {
        let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
        ctx.cast_actor("Alice");
        // A second cast under the same name would alias two identities to one
        // name; the guard is shared across the whole cast_* vocabulary.
        ctx.cast_actor_with_sol("Alice", 1);
    }

    #[test]
    #[should_panic(expected = "build the program first")]
    fn build_with_program_from_file_missing_path_panics_with_guidance() {
        let _ = AnchorLiteSVM::build_with_program_from_file(
            Pubkey::new_unique(),
            "ghost",
            "/nonexistent/path/to/ghost.so",
        );
    }

    #[test]
    #[should_panic(expected = "entrypoint-less stub")]
    fn build_with_program_from_file_stub_elf_panics() {
        // An ELF under 4 KiB is the entrypoint-less-stub signature; the helper
        // names that rather than letting the loader fail opaquely later.
        let path = std::env::temp_dir().join("anchor_litesvm_build_from_file_stub.so");
        std::fs::write(&path, vec![0u8; 100]).unwrap();
        let _ = AnchorLiteSVM::build_with_program_from_file(
            Pubkey::new_unique(),
            "stub",
            path.to_str().unwrap(),
        );
    }

    #[test]
    fn test_full_workflow() {
        // Create test context
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let _ctx = AnchorContext::new(svm, program_id);

        // Test instruction building
        // In anchor 1.0.0, AnchorSerialize is an alias for BorshSerialize
        #[derive(BorshSerialize)]
        struct TestArgs {
            value: u64,
        }

        let accounts = vec![
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ];

        let instruction =
            build_anchor_instruction(&program_id, "test", accounts, TestArgs { value: 42 })
                .unwrap();

        assert_eq!(instruction.program_id, program_id);
        assert!(!instruction.data.is_empty());
    }
}
