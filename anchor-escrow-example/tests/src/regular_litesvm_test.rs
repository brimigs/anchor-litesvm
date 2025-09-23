use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use borsh::BorshSerialize;
use sha2::{Digest, Sha256};
use solana_program_pack::Pack;

#[derive(Debug, BorshSerialize)]
struct MakeArgs {
    seed: u64,
    receive: u64,
    amount: u64,
}

#[derive(Debug, BorshSerialize)]
#[allow(dead_code)]
struct TakeArgs {
    // Take instruction has no arguments
}

#[test]
fn test_take_with_regular_litesvm() {
    // Initialize the test environment
    let mut svm = LiteSVM::new();

    // Deploy your program
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    svm.add_program(program_id, program_bytes);

    // Create and fund test accounts
    let maker = Keypair::new();
    let taker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&taker.pubkey(), 10_000_000_000).unwrap();

    // Create two token mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    // Use litesvm-token to create mints
    use litesvm_token::spl_token;

    // Create mint A
    let create_mint_a_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_a.pubkey(),
        &maker.pubkey(),
        None,
        9, // decimals
    ).unwrap();

    // Create mint B
    let create_mint_b_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_b.pubkey(),
        &maker.pubkey(),
        None,
        9, // decimals
    ).unwrap();

    // First create the mint accounts
    let rent = svm.minimum_balance_for_rent_exemption(82);
    let create_mint_a_account_ix = solana_sdk::system_instruction::create_account(
        &maker.pubkey(),
        &mint_a.pubkey(),
        rent,
        82,
        &spl_token::id(),
    );
    let create_mint_b_account_ix = solana_sdk::system_instruction::create_account(
        &maker.pubkey(),
        &mint_b.pubkey(),
        rent,
        82,
        &spl_token::id(),
    );

    // Create mints transaction
    let tx = Transaction::new_signed_with_payer(
        &[create_mint_a_account_ix, create_mint_a_ix, create_mint_b_account_ix, create_mint_b_ix],
        Some(&maker.pubkey()),
        &[&maker, &mint_a, &mint_b],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Create maker's associated token account for mint_a
    let maker_ata_a = get_associated_token_address(&maker.pubkey(), &mint_a.pubkey());
    let create_maker_ata_a_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &maker.pubkey(),
        &maker.pubkey(),
        &mint_a.pubkey(),
        &spl_token::id(),
    );

    // Create taker's associated token account for mint_b
    let taker_ata_b = get_associated_token_address(&taker.pubkey(), &mint_b.pubkey());
    let create_taker_ata_b_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &taker.pubkey(),
        &taker.pubkey(),
        &mint_b.pubkey(),
        &spl_token::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_maker_ata_a_ix, create_taker_ata_b_ix],
        Some(&maker.pubkey()),
        &[&maker, &taker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Mint tokens to maker's ATA (mint_a)
    let mint_to_maker_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_a.pubkey(),
        &maker_ata_a,
        &maker.pubkey(),
        &[],
        1_000_000_000, // 1 token with 9 decimals
    ).unwrap();

    // Mint tokens to taker's ATA (mint_b)
    let mint_to_taker_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_b.pubkey(),
        &taker_ata_b,
        &maker.pubkey(), // maker is mint authority
        &[],
        500_000_000, // 0.5 tokens with 9 decimals
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_maker_ix, mint_to_taker_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // First, create the escrow with the make instruction
    let seed: u64 = 42;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &program_id,
    );

    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // Build make instruction discriminator
    let mut hasher = Sha256::new();
    hasher.update(b"global:make");
    let hash = hasher.finalize();
    let mut make_discriminator = [0u8; 8];
    make_discriminator.copy_from_slice(&hash[..8]);

    // Serialize make instruction arguments
    let make_args = MakeArgs {
        seed,
        receive: 500_000_000, // 0.5 tokens
        amount: 1_000_000_000, // 1 token
    };

    let mut make_instruction_data = make_discriminator.to_vec();
    make_instruction_data.extend_from_slice(&seed.to_le_bytes());
    make_instruction_data.extend_from_slice(&make_args.receive.to_le_bytes());
    make_instruction_data.extend_from_slice(&make_args.amount.to_le_bytes());

    // Build the make instruction
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
        data: make_instruction_data,
    };

    // Send make transaction
    let tx = Transaction::new_signed_with_payer(
        &[make_instruction],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("Escrow created successfully");

    // Now test the take instruction
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    // Build take instruction discriminator
    let mut hasher = Sha256::new();
    hasher.update(b"global:take");
    let hash = hasher.finalize();
    let mut take_discriminator = [0u8; 8];
    take_discriminator.copy_from_slice(&hash[..8]);

    // Take instruction has no arguments, just the discriminator
    let take_instruction_data = take_discriminator.to_vec();

    // Build the take instruction with all required accounts
    let take_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),   // taker
            AccountMeta::new(maker.pubkey(), false),  // maker
            AccountMeta::new(escrow_pda, false),      // escrow
            AccountMeta::new_readonly(mint_a.pubkey(), false), // mint_a
            AccountMeta::new_readonly(mint_b.pubkey(), false), // mint_b
            AccountMeta::new(vault, false),           // vault
            AccountMeta::new(taker_ata_a, false),     // taker_ata_a
            AccountMeta::new(taker_ata_b, false),     // taker_ata_b
            AccountMeta::new(maker_ata_b, false),     // maker_ata_b
            AccountMeta::new_readonly(spl_associated_token_account::id(), false), // associated_token_program
            AccountMeta::new_readonly(spl_token::id(), false), // token_program
            AccountMeta::new_readonly(system_program::id(), false), // system_program
        ],
        data: take_instruction_data,
    };

    // Build and send take transaction
    let tx = Transaction::new_signed_with_payer(
        &[take_instruction],
        Some(&taker.pubkey()),
        &[&taker],
        svm.latest_blockhash(),
    );

    // Execute and verify
    let result = svm.send_transaction(tx);

    match result {
        Ok(res) => {
            println!("\nTake transaction succeeded!");

            println!("\nTransaction logs:");
            for log in &res.logs {
                println!("  {}", log);
            }

            // Verify escrow account was closed
            // In LiteSVM, closed accounts might still exist with 0 lamports and 0 data
            let escrow_closed = match svm.get_account(&escrow_pda) {
                None => true,
                Some(account) => account.lamports == 0 && account.data.is_empty(),
            };
            assert!(escrow_closed, "Escrow account should be closed (0 lamports, 0 data)");
            println!("\nEscrow account closed successfully");

            // Verify vault account was closed
            let vault_closed = match svm.get_account(&vault) {
                None => true,
                Some(account) => account.lamports == 0 && account.data.is_empty(),
            };
            assert!(vault_closed, "Vault account should be closed (0 lamports, 0 data)");
            println!("Vault account closed successfully");

            // Check final token balances
            use litesvm_token::spl_token;

            // Taker should have received tokens from mint_a
            let taker_ata_a_data = svm.get_account(&taker_ata_a).unwrap();
            let taker_ata_a_state = spl_token::state::Account::unpack(&taker_ata_a_data.data).unwrap();
            assert_eq!(taker_ata_a_state.amount, 1_000_000_000, "Taker should have received 1 token from mint_a");
            println!("Taker received {} tokens from mint_a", taker_ata_a_state.amount as f64 / 1_000_000_000.0);

            // Taker should have sent tokens from mint_b
            let taker_ata_b_data = svm.get_account(&taker_ata_b).unwrap();
            let taker_ata_b_state = spl_token::state::Account::unpack(&taker_ata_b_data.data).unwrap();
            assert_eq!(taker_ata_b_state.amount, 0, "Taker should have sent all tokens from mint_b");
            println!("Taker has {} tokens from mint_b (after sending)", taker_ata_b_state.amount);

            // Maker should have received tokens from mint_b
            let maker_ata_b_data = svm.get_account(&maker_ata_b).unwrap();
            let maker_ata_b_state = spl_token::state::Account::unpack(&maker_ata_b_data.data).unwrap();
            assert_eq!(maker_ata_b_state.amount, 500_000_000, "Maker should have received 0.5 tokens from mint_b");
            println!("Maker received {} tokens from mint_b", maker_ata_b_state.amount as f64 / 1_000_000_000.0);

            println!("\nTake instruction test passed successfully!");
        }
        Err(e) => {
            panic!("Take transaction failed: {:?}", e);
        }
    }
}