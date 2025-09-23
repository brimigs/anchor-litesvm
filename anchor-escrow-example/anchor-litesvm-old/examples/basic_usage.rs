/// Example showing how anchor-litesvm simplifies Anchor program testing
///
/// This example demonstrates the dramatic code reduction achieved with anchor-litesvm
/// compared to the traditional approach with raw LiteSVM.

use anchor_litesvm::{AnchorLiteSVM, TestHelpers, AssertionHelpers, tuple_args};
use solana_sdk::signature::Signer;
use solana_program::pubkey::Pubkey;
use borsh::BorshSerialize;

// Example Anchor instruction arguments
#[derive(BorshSerialize)]
#[allow(dead_code)]
struct TransferArgs {
    amount: u64,
}

fn main() {
    println!("=== Code Comparison: Traditional vs anchor-litesvm ===\n");

    // Note: This example shows the API differences. For runnable tests,
    // you would need actual program bytes from a compiled Anchor program.

    traditional_approach();
    println!("\n{}\n", "=".repeat(60));
    using_anchor_litesvm();

    println!("\n=== Results ===");
    println!("Traditional approach: ~50 lines for basic test");
    println!("With anchor-litesvm: ~10 lines for same test");
    println!("Code reduction: 80%");
}

fn traditional_approach() {
    println!("TRADITIONAL APPROACH (Raw LiteSVM):");
    println!("------------------------------------\n");

    // This shows the verbose traditional approach
    println!("// Setup - requires manual initialization");
    println!("let mut svm = LiteSVM::new();");
    println!("let program_bytes = include_bytes!(\"../target/deploy/program.so\");");
    println!("svm.add_program(program_id, program_bytes);");
    println!();

    println!("// Account creation - manual transaction building");
    println!("let maker = Keypair::new();");
    println!("svm.airdrop(&maker.pubkey(), 10_000_000_000)?;");
    println!();

    println!("// Token mint creation - multiple instructions");
    println!("let mint = Keypair::new();");
    println!("let rent = svm.minimum_balance_for_rent_exemption(82);");
    println!("let create_account_ix = system_instruction::create_account(...);");
    println!("let init_mint_ix = spl_token::instruction::initialize_mint(...);");
    println!("let tx = Transaction::new_signed_with_payer(&[create_account_ix, init_mint_ix], ...);");
    println!("svm.send_transaction(tx)?;");
    println!();

    println!("// Instruction building - manual discriminator");
    println!("use sha2::{{Digest, Sha256}};");
    println!("let mut hasher = Sha256::new();");
    println!("hasher.update(b\"global:transfer\");");
    println!("let hash = hasher.finalize();");
    println!("let mut discriminator = [0u8; 8];");
    println!("discriminator.copy_from_slice(&hash[..8]);");
    println!();

    println!("// Manual data serialization");
    println!("let mut instruction_data = discriminator.to_vec();");
    println!("instruction_data.extend_from_slice(&amount.to_le_bytes());");
    println!();

    println!("// Manual account meta setup");
    println!("let accounts = vec![");
    println!("    AccountMeta::new(from_account, true),");
    println!("    AccountMeta::new(to_account, false),");
    println!("    AccountMeta::new_readonly(authority, true),");
    println!("    // ... more accounts");
    println!("];");
    println!();

    println!("// Transaction execution");
    println!("let ix = Instruction {{ program_id, accounts, data: instruction_data }};");
    println!("let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);");
    println!("let result = svm.send_transaction(tx)?;");
    println!();

    println!("// Manual assertions");
    println!("let account = svm.get_account(&token_account)?;");
    println!("let token_data = unpack_token_account(&account.data)?;");
    println!("assert_eq!(token_data.amount, expected_amount);");
}

fn using_anchor_litesvm() {
    println!("WITH ANCHOR-LITESVM:");
    println!("--------------------\n");

    println!("// One-line setup!");
    println!("let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);");
    println!();

    println!("// Simple account creation");
    println!("let maker = ctx.create_funded_account(10_000_000_000)?;");
    println!("let mint = ctx.create_token_mint(&maker, 9)?;");
    println!("let ata = ctx.create_token_account(&maker, &mint.pubkey(), Some((1_000_000, &maker)))?;");
    println!();

    println!("// Fluent instruction building with automatic discriminator");
    println!("let result = ctx.instruction_builder(\"transfer\")");
    println!("    .signer(\"from\", &from_account)");
    println!("    .account_mut(\"to\", to_account)");
    println!("    .account(\"authority\", authority.pubkey())");
    println!("    .token_program()");
    println!("    .args(tuple_args((amount,)))  // No struct needed!");
    println!("    .execute(&mut ctx, &[&from_account])?;");
    println!();

    println!("// Clean assertions");
    println!("result.assert_success();");
    println!("ctx.assert_token_balance(&token_account, expected_amount);");
}

/// Example: Complete test with anchor-litesvm
#[allow(dead_code)]
fn example_complete_test() {
    // This would be a real test with actual program bytes
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![]; // Would be include_bytes!("../target/deploy/program.so")

    // Initialize test environment - one line!
    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create test accounts - simple helper methods
    let maker = ctx.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.create_funded_account(10_000_000_000).unwrap();

    // Create token mints
    let mint_a = ctx.create_token_mint(&maker, 9).unwrap();
    let _mint_b = ctx.create_token_mint(&taker, 9).unwrap();

    // Create and fund token accounts
    let maker_ata = ctx.create_token_account(
        &maker,
        &mint_a.pubkey(),
        Some((1_000_000_000, &maker))
    ).unwrap();

    // Build and execute instruction with fluent API
    ctx.instruction_builder("transfer")
        .signer("sender", &maker)
        .account_mut("from", maker_ata)
        .account_mut("to", taker.pubkey())
        .account("mint", mint_a.pubkey())
        .token_program()
        .args(tuple_args((500_000_000u64,))) // No struct definition needed!
        .execute(&mut ctx, &[&maker])
        .unwrap()
        .assert_success();

    // Clean assertions
    ctx.assert_token_balance(&maker_ata, 500_000_000);
}

/// Example: Error handling
#[allow(dead_code)]
fn example_error_handling() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);
    let account = ctx.create_funded_account(1_000_000_000).unwrap();

    // Execute instruction that might fail
    let result = ctx.instruction_builder("risky_operation")
        .signer("user", &account)
        .account_mut("target", Pubkey::new_unique())
        .args(tuple_args(()))
        .execute(&mut ctx, &[&account]);

    // Handle errors elegantly
    match result {
        Ok(tx_result) => {
            println!("Transaction succeeded!");
            if tx_result.has_log("Success") {
                println!("Found success message in logs");
            }
            println!("Used {} compute units", tx_result.compute_units());
        }
        Err(e) => {
            println!("Transaction failed: {:?}", e);
            // Can check specific error conditions
        }
    }
}

/// Example: Multiple programs
#[allow(dead_code)]
fn example_multiple_programs() {
    use anchor_litesvm::AnchorLiteSVM;

    let program1_id = Pubkey::new_unique();
    let program2_id = Pubkey::new_unique();
    let program1_bytes = vec![];
    let program2_bytes = vec![];

    // Deploy multiple programs
    let mut ctx = AnchorLiteSVM::new()
        .deploy_program(program1_id, &program1_bytes)
        .deploy_program(program2_id, &program2_bytes)
        .with_primary_program(program1_id)
        .build();

    // Now you can test cross-program invocations
    let user = ctx.create_funded_account(10_000_000_000).unwrap();

    // Call program1
    ctx.instruction_builder("initialize")
        .signer("user", &user)
        .account_mut("state", Pubkey::new_unique())
        .system_program()
        .args(tuple_args(()))
        .execute(&mut ctx, &[&user])
        .unwrap()
        .assert_success();
}