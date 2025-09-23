/// Example demonstrating the new workspace structure with separate crates
///
/// This example shows how litesvm-utils provides general testing utilities
/// while anchor-litesvm adds Anchor-specific functionality on top.

use anchor_litesvm::{AnchorLiteSVM, tuple_args, AnchorContext};
use litesvm_utils::{TestHelpers, AssertionHelpers, TransactionHelpers};
use solana_sdk::signature::{Keypair, Signer};
use solana_program::pubkey::Pubkey;

fn main() {
    println!("=== New Workspace Structure Demo ===\n");

    println!("The codebase is now split into two crates:\n");

    println!("1. litesvm-utils: General testing utilities");
    println!("   - Account creation and funding");
    println!("   - Token operations (mints, accounts, minting)");
    println!("   - Transaction helpers");
    println!("   - Assertion helpers");
    println!("   - PDA derivation");
    println!("   - Framework agnostic - works with any Solana program\n");

    println!("2. anchor-litesvm: Anchor-specific integration");
    println!("   - Anchor account deserialization");
    println!("   - Automatic discriminator calculation");
    println!("   - Fluent instruction builder with tuple arguments");
    println!("   - Integration path for anchor-client (future)\n");

    example_litesvm_utils();
    println!("\n{}\n", "=".repeat(60));
    example_anchor_litesvm();
}

fn example_litesvm_utils() {
    println!("USING LITESVM-UTILS (General Utilities):");
    println!("-----------------------------------------\n");

    println!("// Framework-agnostic test setup");
    println!("use litesvm_utils::{{LiteSVMBuilder, TestHelpers, AssertionHelpers}};");
    println!();

    println!("// Build test environment");
    println!("let mut svm = LiteSVMBuilder::new()");
    println!("    .deploy_program(program_id, program_bytes)");
    println!("    .build();");
    println!();

    println!("// Use general test helpers");
    println!("let maker = svm.create_funded_account(10_000_000_000)?;");
    println!("let mint = svm.create_token_mint(&maker, 9)?;");
    println!("let token_account = svm.create_token_account(&mint.pubkey(), &maker)?;");
    println!();

    println!("// Send transaction (framework agnostic)");
    println!("let result = svm.send_instruction(ix, &[&maker])?;");
    println!("result.assert_success();");
    println!();

    println!("// General assertions");
    println!("svm.assert_sol_balance(&maker.pubkey(), 10_000_000_000);");
    println!("svm.assert_token_balance(&token_account.pubkey(), 0);");
}

fn example_anchor_litesvm() {
    println!("USING ANCHOR-LITESVM (Anchor Integration):");
    println!("-------------------------------------------\n");

    println!("// Anchor-specific setup");
    println!("use anchor_litesvm::{{AnchorLiteSVM, tuple_args}};");
    println!();

    println!("// Build Anchor test environment");
    println!("let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);");
    println!();

    println!("// Fluent Anchor instruction builder");
    println!("let result = ctx.instruction_builder(\"transfer\")");
    println!("    .signer(\"from\", &maker)");
    println!("    .account_mut(\"to\", recipient)");
    println!("    .token_program()");
    println!("    .args(tuple_args((100u64,)))  // No struct needed!");
    println!("    .execute(&mut ctx, &[&maker])?;");
    println!();

    println!("// Get Anchor accounts with automatic deserialization");
    println!("let account: MyAnchorAccount = ctx.get_account(&account_pubkey)?;");
    println!();

    println!("// Future: anchor-client compatible syntax (WIP)");
    println!("// let program = client.program(program_id);");
    println!("// program.request()");
    println!("//     .accounts(my_program::accounts::Transfer {{ ... }})");
    println!("//     .args(my_program::instruction::Transfer {{ amount }})");
    println!("//     .send()?;");
}

#[cfg(test)]
mod tests {
    use super::*;
    use litesvm::LiteSVM;

    #[test]
    fn test_workspace_structure() {
        // Test that we can use litesvm-utils independently
        let mut svm = LiteSVM::new();
        let account = svm.create_funded_account(1_000_000_000).unwrap();
        svm.assert_sol_balance(&account.pubkey(), 1_000_000_000);

        // Test that we can use anchor-litesvm for Anchor programs
        let program_id = Pubkey::new_unique();
        let ctx = AnchorContext::new(svm, program_id);
        let discriminator = anchor_litesvm::calculate_anchor_discriminator("test");
        assert_eq!(discriminator.len(), 8);
    }
}