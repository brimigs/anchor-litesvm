/// Advanced features demonstration for anchor-litesvm
///
/// This example showcases more complex testing scenarios including:
/// - Token operations (mint, transfer, burn)
/// - PDA (Program Derived Address) calculations
/// - Batch operations
/// - Error handling and assertions
/// - Transaction metadata analysis

use anchor_litesvm::{
    AnchorLiteSVM, TestHelpers, AssertionHelpers, tuple_args,
};
use solana_sdk::signature::{Keypair, Signer};
use solana_program::pubkey::Pubkey;

fn main() {
    println!("=== Advanced Features of anchor-litesvm ===\n");

    // Note: These examples demonstrate the API. For runnable tests,
    // you would need actual compiled Anchor program bytes.

    println!("1. Token Operations");
    println!("2. PDA Calculations");
    println!("3. Batch Operations");
    println!("4. Advanced Assertions");
    println!("5. Transaction Analysis");
    println!("6. Error Recovery");

    println!("\nFor complete working examples, see the test files in the repository.");
}

/// Example: Comprehensive token operations
#[allow(dead_code)]
fn example_token_operations() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![]; // Would be actual program bytes

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create participants
    let alice = ctx.create_funded_account(10_000_000_000).unwrap();
    let bob = ctx.create_funded_account(10_000_000_000).unwrap();
    let charlie = ctx.create_funded_account(10_000_000_000).unwrap();

    // Create a token mint with Alice as authority
    let mint = ctx.create_token_mint(&alice, 9).unwrap();

    // Create token accounts for each participant
    let alice_ata = ctx.create_token_account(
        &alice,
        &mint.pubkey(),
        Some((1_000_000_000_000, &alice)) // Mint 1000 tokens to Alice
    ).unwrap();

    let bob_ata = ctx.create_token_account(
        &bob,
        &mint.pubkey(),
        None // No initial balance
    ).unwrap();

    let charlie_ata = ctx.create_token_account(
        &charlie,
        &mint.pubkey(),
        None
    ).unwrap();

    // Transfer tokens from Alice to Bob
    ctx.instruction_builder("transfer")
        .signer("from", &alice)
        .account_mut("from_ata", alice_ata)
        .account_mut("to_ata", bob_ata)
        .account("mint", mint.pubkey())
        .token_program()
        .args(tuple_args((250_000_000_000u64,))) // Transfer 250 tokens
        .execute(&mut ctx, &[&alice])
        .unwrap()
        .assert_success();

    // Verify balances
    ctx.assert_token_balance(&alice_ata, 750_000_000_000);
    ctx.assert_token_balance(&bob_ata, 250_000_000_000);

    // Batch transfer from Bob to multiple recipients
    let transfers = vec![
        (bob_ata, charlie_ata, 100_000_000_000u64),
        (bob_ata, alice_ata, 50_000_000_000u64),
    ];

    for (from, to, amount) in transfers {
        ctx.instruction_builder("transfer")
            .signer("authority", &bob)
            .account_mut("from", from)
            .account_mut("to", to)
            .account("mint", mint.pubkey())
            .token_program()
            .args(tuple_args((amount,)))
            .execute(&mut ctx, &[&bob])
            .unwrap();
    }

    // Final balance assertions
    ctx.assert_token_balance(&alice_ata, 800_000_000_000);
    ctx.assert_token_balance(&bob_ata, 100_000_000_000);
    ctx.assert_token_balance(&charlie_ata, 100_000_000_000);
}

/// Example: Working with PDAs (Program Derived Addresses)
#[allow(dead_code)]
fn example_pda_operations() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create user account
    let user = ctx.create_funded_account(10_000_000_000).unwrap();

    // Calculate PDA with seeds
    let seed = 42u64;
    let (pda, bump) = ctx.find_pda(&[
        b"user_vault",
        user.pubkey().as_ref(),
        &seed.to_le_bytes()
    ]);

    println!("PDA: {}", pda);
    println!("Bump: {}", bump);

    // Initialize PDA account
    ctx.instruction_builder("initialize_vault")
        .signer("user", &user)
        .account_mut("vault", pda)
        .account("user", user.pubkey())
        .system_program()
        .args(tuple_args((seed, bump)))
        .execute(&mut ctx, &[&user])
        .unwrap()
        .assert_success();

    // Verify PDA was created
    ctx.assert_account_exists(&pda);
    ctx.assert_account_owner(&pda, &program_id);

    // Use PDA in another instruction
    ctx.instruction_builder("deposit_to_vault")
        .signer("user", &user)
        .account_mut("vault", pda)
        .account_mut("user", user.pubkey())
        .system_program()
        .args(tuple_args((1_000_000_000u64,))) // Deposit 1 SOL
        .execute(&mut ctx, &[&user])
        .unwrap()
        .assert_success();

    // Verify deposit
    ctx.assert_account_lamports(&pda, 1_000_000_000);
}

/// Example: Batch account creation and operations
#[allow(dead_code)]
fn example_batch_operations() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create multiple accounts at once
    let accounts = ctx.create_funded_accounts(10, 1_000_000_000).unwrap();
    println!("Created {} test accounts", accounts.len());

    // Create multiple mints
    let mint_authority = &accounts[0];
    let mints: Vec<Keypair> = (0..5)
        .map(|_| ctx.create_token_mint(mint_authority, 9).unwrap())
        .collect();

    // Create token accounts for each user for each mint
    for account in &accounts[1..6] {
        for mint in &mints {
            let ata = ctx.create_token_account(
                account,
                &mint.pubkey(),
                Some((100_000_000, mint_authority))
            ).unwrap();

            // Verify creation
            ctx.assert_token_balance(&ata, 100_000_000);
        }
    }

    // Batch airdrop to multiple accounts
    let recipient_pubkeys: Vec<Pubkey> = accounts.iter()
        .map(|a| a.pubkey())
        .collect();
    let recipients: Vec<&Pubkey> = recipient_pubkeys.iter().collect();

    ctx.batch_airdrop(&recipients[..5], 500_000_000).unwrap();

    // Verify airdrops
    for account in &accounts[..5] {
        let balance = ctx.svm.get_balance(&account.pubkey()).unwrap();
        assert!(balance >= 1_500_000_000); // Original + airdrop
    }
}

/// Example: Advanced assertions and verifications
#[allow(dead_code)]
fn example_advanced_assertions() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    let user = ctx.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.create_token_mint(&user, 9).unwrap();
    let token_account = ctx.create_token_account(
        &user,
        &mint.pubkey(),
        Some((1_000_000_000, &user))
    ).unwrap();

    // Execute some operation
    let result = ctx.instruction_builder("process")
        .signer("user", &user)
        .account_mut("token_account", token_account)
        .token_program()
        .args(tuple_args(()))
        .execute(&mut ctx, &[&user])
        .unwrap();

    // Transaction result assertions
    result.assert_success();
    assert!(result.has_log("Processing started"));
    assert!(result.has_log("Processing completed"));

    // Compute units check
    let compute_units = result.compute_units();
    assert!(compute_units < 200_000, "Operation used too many compute units");

    // Account assertions
    ctx.assert_account_exists(&token_account);
    ctx.assert_account_owner(&token_account, &spl_token::id());
    ctx.assert_token_balance(&token_account, 1_000_000_000);

    // Custom message assertions
    ctx.assert_token_balance_with_msg(
        &token_account,
        1_000_000_000,
        "Token balance should remain unchanged"
    );

    // Multiple account closure check
    let temp_account1 = Pubkey::new_unique();
    let temp_account2 = Pubkey::new_unique();

    ctx.instruction_builder("cleanup")
        .signer("user", &user)
        .account_mut("temp1", temp_account1)
        .account_mut("temp2", temp_account2)
        .args(tuple_args(()))
        .execute(&mut ctx, &[&user])
        .unwrap();

    ctx.assert_accounts_closed(&[&temp_account1, &temp_account2]);
}

/// Example: Transaction analysis and debugging
#[allow(dead_code)]
fn example_transaction_analysis() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);
    let user = ctx.create_funded_account(10_000_000_000).unwrap();

    // Execute a complex transaction
    let result = ctx.instruction_builder("complex_operation")
        .signer("user", &user)
        .account_mut("state", Pubkey::new_unique())
        .system_program()
        .args(tuple_args((100u64, 200u64, 300u64)))
        .execute(&mut ctx, &[&user])
        .unwrap();

    // Analyze transaction
    println!("Transaction Analysis:");
    // Transaction succeeded if we have a result
    println!("  Success: true");
    println!("  Compute Units: {}", result.compute_units());

    // Check for specific log messages
    let log_patterns = [
        "Initializing state",
        "Validating parameters",
        "Processing data",
        "Finalizing transaction",
    ];

    for pattern in &log_patterns {
        if result.has_log(pattern) {
            println!("  Found: {}", pattern);
        } else {
            println!("  Missing: {}", pattern);
        }
    }

    // Access logs
    let logs = result.logs();
    println!("  Log Messages: {} lines", logs.len());

    // Extract specific values from logs (if your program logs them)
    for log in logs {
        if log.contains("Result:") {
            println!("  {}", log);
        }
    }
}

/// Example: Error handling and recovery
#[allow(dead_code)]
fn example_error_recovery() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);

    // Create accounts with different balances
    let rich_user = ctx.create_funded_account(10_000_000_000).unwrap();
    let poor_user = ctx.create_funded_account(100).unwrap(); // Very low balance

    // Attempt operation that requires more SOL than available
    let result = ctx.instruction_builder("expensive_operation")
        .signer("user", &poor_user)
        .account_mut("state", Pubkey::new_unique())
        .system_program()
        .args(tuple_args((1_000_000_000u64,))) // Requires 1 SOL
        .execute(&mut ctx, &[&poor_user]);

    // Handle the error
    match result {
        Ok(_) => panic!("Should have failed due to insufficient funds"),
        Err(e) => {
            println!("Expected error: {:?}", e);

            // Recover by funding the account
            ctx.svm.airdrop(&poor_user.pubkey(), 2_000_000_000).unwrap();

            // Retry the operation
            let retry_result = ctx.instruction_builder("expensive_operation")
                .signer("user", &poor_user)
                .account_mut("state", Pubkey::new_unique())
                .system_program()
                .args(tuple_args((1_000_000_000u64,)))
                .execute(&mut ctx, &[&poor_user]);

            // Should succeed now
            retry_result.unwrap().assert_success();
            println!("Operation succeeded after funding account");
        }
    }

    // Test custom program errors
    let invalid_amount = u64::MAX;
    let result = ctx.instruction_builder("validate_amount")
        .signer("user", &rich_user)
        .args(tuple_args((invalid_amount,)))
        .execute(&mut ctx, &[&rich_user]);

    if let Err(e) = result {
        // Check for specific error codes or messages
        let error_string = format!("{:?}", e);
        if error_string.contains("AmountTooLarge") {
            println!("Caught expected validation error");
        }
    }
}

/// Example: Cross-program invocation testing
#[allow(dead_code)]
fn example_cross_program_invocation() {
    // Deploy multiple programs
    let _token_program_id = spl_token::id();
    let oracle_program_id = Pubkey::new_unique();
    let main_program_id = Pubkey::new_unique();

    let oracle_bytes = vec![]; // Oracle program bytes
    let main_bytes = vec![]; // Main program bytes

    let mut ctx = AnchorLiteSVM::new()
        .deploy_program(oracle_program_id, &oracle_bytes)
        .deploy_program(main_program_id, &main_bytes)
        .with_primary_program(main_program_id)
        .build();

    let user = ctx.create_funded_account(10_000_000_000).unwrap();

    // Initialize oracle
    let oracle_state = Pubkey::new_unique();
    ctx.instruction_builder("initialize_oracle")
        .signer("authority", &user)
        .account_mut("state", oracle_state)
        .system_program()
        .args(tuple_args(()))
        .execute(&mut ctx, &[&user])
        .unwrap()
        .assert_success();

    // Main program calls oracle
    let result = ctx.instruction_builder("get_price_and_swap")
        .signer("user", &user)
        .account("oracle", oracle_state)
        .account("oracle_program", oracle_program_id)
        .token_program()
        .args(tuple_args((1_000_000u64,)))
        .execute(&mut ctx, &[&user])
        .unwrap();

    // Verify cross-program invocation succeeded
    assert!(result.has_log("CPI to oracle program"));
    assert!(result.has_log("Oracle returned price"));
    result.assert_success();
}

/// Example: Testing with time-based logic
#[allow(dead_code)]
fn example_time_based_testing() {
    let program_id = Pubkey::new_unique();
    let program_bytes = vec![];

    let mut ctx = AnchorLiteSVM::build_with_program(program_id, &program_bytes);
    let user = ctx.create_funded_account(10_000_000_000).unwrap();

    // Create a time-locked vault
    let vault = Pubkey::new_unique();
    let unlock_time = 1_700_000_000i64; // Some future timestamp

    ctx.instruction_builder("create_timelock")
        .signer("user", &user)
        .account_mut("vault", vault)
        .account("user", user.pubkey())
        .system_program()
        .args(tuple_args((unlock_time, 1_000_000_000u64)))
        .execute(&mut ctx, &[&user])
        .unwrap()
        .assert_success();

    // Attempt to withdraw before unlock time
    let early_withdraw = ctx.instruction_builder("withdraw_timelock")
        .signer("user", &user)
        .account_mut("vault", vault)
        .account_mut("user", user.pubkey())
        .args(tuple_args(()))
        .execute(&mut ctx, &[&user]);

    // Should fail
    assert!(early_withdraw.is_err(), "Withdrawal should fail before unlock time");

    // Note: In a real test, you would need to manipulate the clock
    // or wait for the appropriate time. LiteSVM may have utilities for this.

    // After time has passed (simulated)
    // This would normally require clock manipulation
    let _late_withdraw = ctx.instruction_builder("withdraw_timelock")
        .signer("user", &user)
        .account_mut("vault", vault)
        .account_mut("user", user.pubkey())
        .args(tuple_args(()))
        .execute(&mut ctx, &[&user]);

    // In a real test with proper time manipulation, this would succeed
    // For this example, it demonstrates the pattern
}