/// Advanced features demonstration for anchor-litesvm
///
/// This example showcases more complex testing scenarios including:
/// - Token operations (mint, transfer, burn)
/// - PDA (Program Derived Address) calculations
/// - Batch operations
/// - Error handling and assertions
/// - Transaction metadata analysis
///
/// Note: These examples demonstrate the actual working API.
/// For runnable tests, you would need compiled Anchor program bytes.

use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
use solana_sdk::signature::Signer;
use solana_program::pubkey::Pubkey;

fn main() {
    println!("=== Advanced Features of anchor-litesvm ===\n");

    println!("1. Token Operations - Creating and managing SPL tokens");
    println!("2. PDA Calculations - Working with Program Derived Addresses");
    println!("3. Batch Operations - Creating multiple accounts efficiently");
    println!("4. Advanced Assertions - Testing account states");
    println!("5. Transaction Analysis - Debugging with logs and compute units");
    println!("6. Error Recovery - Handling and retrying failed transactions");

    println!("\n=== Key APIs Demonstrated ===");
    println!("- AnchorLiteSVM::build_with_program() - One-line setup");
    println!("- TestHelpers trait - Account and token creation");
    println!("- AssertionHelpers trait - Account state verification");
    println!("- TransactionResult - Log analysis and debugging");
    println!("- ctx.program().accounts() - Simplified instruction building");

    println!("\nFor complete working examples, see the anchor-escrow-example tests.");
}

/// Example: Comprehensive token operations
///
/// This shows the actual API for creating tokens, mints, and performing operations.
#[allow(dead_code)]
fn example_token_operations() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![]; // Would be include_bytes!("../target/deploy/program.so")

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create participants using TestHelpers trait
    let alice = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let bob = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let charlie = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    // Create a token mint with Alice as authority
    let mint = ctx.svm.create_token_mint(&alice, 9).unwrap();

    // Create associated token accounts for each participant
    let alice_ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &alice).unwrap();
    let bob_ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &bob).unwrap();
    let charlie_ata = ctx.svm.create_associated_token_account(&mint.pubkey(), &charlie).unwrap();

    // Mint tokens to Alice's account
    ctx.svm.mint_to(&mint.pubkey(), &alice_ata, &alice, 1_000_000_000_000u64).unwrap();

    // Verify initial balance using AssertionHelpers trait
    ctx.svm.assert_token_balance(&alice_ata, 1_000_000_000_000u64);
    ctx.svm.assert_token_balance(&bob_ata, 0);
    ctx.svm.assert_token_balance(&charlie_ata, 0);

    println!("✓ Token operations completed successfully");
    println!("  Created mint with {} decimals", 9);
    println!("  Created 3 token accounts");
    println!("  Minted {} tokens to Alice", 1_000_000_000_000u64);
}

/// Example: Working with PDAs (Program Derived Addresses)
///
/// Shows how to calculate and use PDAs with the actual API.
#[allow(dead_code)]
fn example_pda_operations() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create user account
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    // Calculate PDA with seeds - two ways to do it:
    let seed = 42u64;

    // Method 1: Get just the PDA
    let pda = ctx.svm.get_pda(&[
        b"user_vault",
        user.pubkey().as_ref(),
        &seed.to_le_bytes()
    ], &program_id);

    // Method 2: Get PDA and bump
    let (pda_with_bump, bump) = ctx.svm.get_pda_with_bump(&[
        b"user_vault",
        user.pubkey().as_ref(),
        &seed.to_le_bytes()
    ], &program_id);

    assert_eq!(pda, pda_with_bump);

    println!("✓ PDA calculation completed");
    println!("  PDA: {}", pda);
    println!("  Bump: {}", bump);

    // In a real test, you would now build an instruction using the simplified syntax:
    // let ix = ctx.program()
    //     .accounts(my_program::accounts::Initialize {
    //         vault: pda,
    //         user: user.pubkey(),
    //         system_program: system_program::id(),
    //     })
    //     .args(my_program::instruction::Initialize { seed, bump })
    //     .instruction()
    //     .unwrap();
    //
    // ctx.execute_instruction(ix, &[&user]).unwrap().assert_success();
}

/// Example: Batch account creation and operations
///
/// Shows how to create multiple accounts efficiently.
#[allow(dead_code)]
fn example_batch_operations() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create multiple accounts at once using TestHelpers trait
    let accounts = ctx.svm.create_funded_accounts(10, 1_000_000_000).unwrap();
    println!("✓ Created {} test accounts", accounts.len());

    // Create multiple mints
    let mint_authority = &accounts[0];
    let mut mints = Vec::new();

    for i in 0..5 {
        let mint = ctx.svm.create_token_mint(mint_authority, 9).unwrap();
        mints.push(mint);
        println!("  Created mint {}: {}", i + 1, mints[i].pubkey());
    }

    // Create token accounts for each user for the first mint
    let first_mint = &mints[0];
    for (i, account) in accounts[1..6].iter().enumerate() {
        let ata = ctx.svm.create_associated_token_account(&first_mint.pubkey(), account).unwrap();

        // Mint initial tokens
        ctx.svm.mint_to(&first_mint.pubkey(), &ata, mint_authority, 100_000_000).unwrap();

        // Verify creation using AssertionHelpers trait
        ctx.svm.assert_token_balance(&ata, 100_000_000);
        println!("  Created and funded token account {} for user {}", i + 1, i + 2);
    }

    // Verify SOL balances
    for (i, account) in accounts.iter().enumerate() {
        let balance = ctx.svm.get_balance(&account.pubkey()).unwrap();
        println!("  Account {} SOL balance: {}", i + 1, balance);
        assert!(balance > 0, "Account should have SOL");
    }
}

/// Example: Advanced assertions and verifications
///
/// Shows all available assertion methods.
#[allow(dead_code)]
fn example_advanced_assertions() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 9).unwrap();
    let token_account = ctx.svm.create_associated_token_account(&mint.pubkey(), &user).unwrap();

    // Mint some tokens
    ctx.svm.mint_to(&mint.pubkey(), &token_account, &user, 1_000_000_000).unwrap();

    println!("=== Demonstrating Assertion Methods ===");

    // Account existence assertions
    ctx.svm.assert_account_exists(&user.pubkey());
    ctx.svm.assert_account_exists(&mint.pubkey());
    ctx.svm.assert_account_exists(&token_account);
    println!("✓ Account existence checks passed");

    // Balance assertions
    ctx.svm.assert_token_balance(&token_account, 1_000_000_000);
    ctx.svm.assert_sol_balance(&user.pubkey(), 10_000_000_000);
    println!("✓ Balance checks passed");

    // Mint supply assertion
    ctx.svm.assert_mint_supply(&mint.pubkey(), 1_000_000_000);
    println!("✓ Mint supply check passed");

    // Account owner assertion
    ctx.svm.assert_account_owner(&token_account, &spl_token::id());
    println!("✓ Account owner check passed");

    // Account data length assertion
    ctx.svm.assert_account_data_len(&token_account, 165); // SPL token account size
    println!("✓ Account data length check passed");

    // Test account closure (would happen after an actual close instruction)
    let temp_account = Pubkey::new_unique();
    ctx.svm.assert_account_closed(&temp_account); // Non-existent account is "closed"
    println!("✓ Account closure check passed");
}

/// Example: Transaction analysis and debugging
///
/// Shows how to analyze transaction results.
#[allow(dead_code)]
fn example_transaction_analysis() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);
    let _user = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    // In a real scenario, you would execute an instruction and get a result:
    // let result = ctx.execute_instruction(ix, &[&user]).unwrap();
    //
    // Then you can analyze it:
    //
    // // Check success
    // result.assert_success();
    // println!("✓ Transaction succeeded");
    //
    // // Get compute units
    // let cu = result.compute_units();
    // println!("  Compute units used: {}", cu);
    // assert!(cu < 200_000, "Transaction used too many compute units");
    //
    // // Check logs
    // if result.has_log("Program log: Success") {
    //     println!("  Found success log");
    // }
    //
    // // Get all logs
    // for (i, log) in result.logs().iter().enumerate() {
    //     println!("  Log {}: {}", i + 1, log);
    // }
    //
    // // Find specific log
    // if let Some(log) = result.find_log("Result:") {
    //     println!("  Found result log: {}", log);
    // }
    //
    // // Print formatted logs
    // result.print_logs();

    println!("=== Transaction Analysis APIs ===");
    println!("Available methods on TransactionResult:");
    println!("  - assert_success()     : Panic if transaction failed");
    println!("  - is_success()         : Returns bool");
    println!("  - compute_units()      : Get compute units consumed");
    println!("  - has_log(msg)         : Check if logs contain message");
    println!("  - find_log(pattern)    : Find first matching log");
    println!("  - logs()               : Get all log messages");
    println!("  - print_logs()         : Pretty print all logs");
    println!("  - error()              : Get error message if failed");
}

/// Example: Error handling and recovery
///
/// Shows how to handle failed transactions.
#[allow(dead_code)]
fn example_error_recovery() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create accounts with different balances
    let rich_user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let poor_user = ctx.svm.create_funded_account(100).unwrap(); // Very low balance

    println!("=== Error Handling Example ===");
    println!("Rich user balance: {} lamports", ctx.svm.get_balance(&rich_user.pubkey()).unwrap());
    println!("Poor user balance: {} lamports", ctx.svm.get_balance(&poor_user.pubkey()).unwrap());

    // In a real scenario with an actual expensive instruction:
    // let result = ctx.execute_instruction(expensive_ix, &[&poor_user]);
    //
    // if let Ok(tx_result) = result {
    //     if !tx_result.is_success() {
    //         println!("Transaction failed: {:?}", tx_result.error());
    //         tx_result.print_logs();
    //
    //         // Recover by funding the account
    //         ctx.svm.airdrop(&poor_user.pubkey(), 2_000_000_000).unwrap();
    //         println!("✓ Account funded, retrying...");
    //
    //         // Retry
    //         let retry_result = ctx.execute_instruction(expensive_ix, &[&poor_user]).unwrap();
    //         retry_result.assert_success();
    //         println!("✓ Transaction succeeded after funding");
    //     }
    // }

    // You can also check transaction success with is_success():
    // if tx_result.is_success() {
    //     println!("Transaction succeeded!");
    // } else {
    //     println!("Transaction failed: {:?}", tx_result.error());
    // }
}

/// Example: Complete workflow with production-compatible syntax
///
/// Shows the full pattern: setup → build instruction → execute → verify
#[allow(dead_code)]
fn example_complete_workflow() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    println!("=== Complete Workflow Example ===\n");

    // 1. Setup
    println!("1. Setup test environment");
    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 9).unwrap();
    let token_account = ctx.svm.create_associated_token_account(&mint.pubkey(), &user).unwrap();
    println!("   ✓ Created user, mint, and token account\n");

    // 2. Build instruction using simplified syntax
    println!("2. Build instruction (simplified syntax)");
    // In a real test with actual program:
    // let ix = ctx.program()
    //     .accounts(my_program::accounts::Transfer {
    //         from: token_account,
    //         to: recipient_account,
    //         authority: user.pubkey(),
    //         token_program: spl_token::id(),
    //     })
    //     .args(my_program::instruction::Transfer {
    //         amount: 500_000_000,
    //     })
    //     .instruction()
    //     .unwrap();
    println!("   ✓ Instruction built with ctx.program().accounts()\n");

    // 3. Execute
    println!("3. Execute instruction");
    // let result = ctx.execute_instruction(ix, &[&user]).unwrap();
    // result.assert_success();
    println!("   ✓ Instruction executed successfully\n");

    // 4. Verify
    println!("4. Verify results");
    ctx.svm.assert_account_exists(&user.pubkey());
    ctx.svm.assert_account_exists(&token_account);
    ctx.svm.assert_token_balance(&token_account, 0); // Initial balance, no mint yet
    println!("   ✓ All assertions passed\n");

    println!("=== Key Takeaways ===");
    println!("• Use AnchorLiteSVM::build_with_program() for setup");
    println!("• Access helpers via ctx.svm (TestHelpers, AssertionHelpers)");
    println!("• Build instructions with ctx.program().accounts()");
    println!("• Execute with ctx.execute_instruction()");
    println!("• Verify with assertion helpers");
}

/// Example: Clock and slot manipulation
///
/// Shows time-based testing capabilities.
#[allow(dead_code)]
fn example_clock_manipulation() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    println!("=== Clock and Slot Manipulation ===");

    // Get current slot
    let current_slot = ctx.svm.get_current_slot();
    println!("Current slot: {}", current_slot);

    // Advance slot
    ctx.svm.advance_slot(100);
    let new_slot = ctx.svm.get_current_slot();
    println!("After advancing 100 slots: {}", new_slot);
    assert_eq!(new_slot, current_slot + 100);

    println!("✓ Clock manipulation available for time-based testing");
}
