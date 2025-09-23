use litesvm::LiteSVM;
use litesvm_utils::TestHelpers;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_program::pubkey::Pubkey;
use anchor_client::{Client, Cluster, Program};
use solana_client::rpc_client::RpcClient;
use std::rc::Rc;
use spl_associated_token_account::get_associated_token_address;
use solana_program_pack::Pack;

// Generate client modules from IDL
anchor_lang::declare_program!(anchor_escrow);

#[test]
fn test_make_with_anchor_client() {
    // Initialize the test environment
    let mut svm = LiteSVM::new();

    // Deploy your program
    let program_id = anchor_escrow::ID;
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    svm.add_program(program_id, program_bytes);

    // Create and fund test accounts
    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

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
    let rent = svm.minimum_balance_for_rent_exemption(82); // Mint::LEN = 82
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
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &maker.pubkey(),
        &maker.pubkey(),
        &mint_a.pubkey(),
        &spl_token::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Mint tokens to maker's ATA
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_a.pubkey(),
        &maker_ata_a,
        &maker.pubkey(),
        &[],
        1_000_000_000, // 1 token with 9 decimals
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Calculate PDAs and addresses using the helper
    let seed: u64 = 42;
    let escrow_pda = svm.get_pda(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &program_id,
    );

    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    let _mock_rpc = RpcClient::new_mock("succeeds");

    let payer_keypair = Keypair::new(); 
    let client = Client::new_with_options(
        Cluster::Custom("http://127.0.0.1:8899".to_string(), "ws://127.0.0.1:8900".to_string()),
        Rc::new(payer_keypair),
        CommitmentConfig::confirmed(),
    );

    let program: Program<Rc<Keypair>> = client.program(program_id).unwrap();

    let make_ix = program
        .request()
        .accounts(anchor_escrow::client::accounts::Make {
            maker: maker.pubkey(),
            escrow: escrow_pda,
            mint_a: mint_a.pubkey(),
            mint_b: mint_b.pubkey(),
            maker_ata_a,
            vault,
            associated_token_program: spl_associated_token_account::id(),
            token_program: spl_token::id(),
            system_program: solana_sdk::system_program::id(),
        })
        .args(anchor_escrow::client::args::Make {
            seed,
            receive: 500_000_000,  // 0.5 tokens
            amount: 1_000_000_000,  // 1 token
        })
        .instructions()
        .unwrap()
        .remove(0);

    // Build and send transaction using LiteSVM
    let tx = Transaction::new_signed_with_payer(
        &[make_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );

    // Execute and verify
    let result = svm.send_transaction(tx);

    match result {
        Ok(res) => {
            println!("Transaction succeeded with anchor_client!");
            println!("\nTransaction logs:");
            for log in &res.logs {
                println!("  {}", log);
            }

            // Verify escrow account was created
            let escrow_account = svm.get_account(&escrow_pda);
            assert!(escrow_account.is_some(), "Escrow account should exist");
            println!("\nEscrow account created at: {}", escrow_pda);

            // Verify vault account was created and has tokens
            let vault_account = svm.get_account(&vault);
            assert!(vault_account.is_some(), "Vault account should exist");
            println!("Vault account created at: {}", vault);

            // Check token balances
            use litesvm_token::spl_token;
            let vault_data = vault_account.unwrap();
            let vault_state = spl_token::state::Account::unpack(&vault_data.data).unwrap();
            assert_eq!(vault_state.amount, 1_000_000_000, "Vault should have 1 token");
            println!("Vault has {} tokens", vault_state.amount as f64 / 1_000_000_000.0);

            let maker_ata_data = svm.get_account(&maker_ata_a).unwrap();
            let maker_ata_state = spl_token::state::Account::unpack(&maker_ata_data.data).unwrap();
            assert_eq!(maker_ata_state.amount, 0, "Maker ATA should have 0 tokens after transfer");
            println!("Maker ATA has {} tokens (after transfer)", maker_ata_state.amount);

            println!("\nMake instruction test with anchor_client passed successfully!");
        }
        Err(e) => {
            panic!("Transaction failed: {:?}", e);
        }
    }
}

#[test]
fn test_take_with_anchor_client() {
    // Initialize the test environment
    let mut svm = LiteSVM::new();

    // Deploy your program
    let program_id = anchor_escrow::ID;
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
    let rent = svm.minimum_balance_for_rent_exemption(82); // Mint::LEN = 82
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
    let escrow_pda = svm.get_pda(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &program_id,
    );

    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    let _mock_rpc = RpcClient::new_mock("succeeds");

    let payer_keypair = Keypair::new();
    let client = Client::new_with_options(
        Cluster::Custom("http://127.0.0.1:8899".to_string(), "ws://127.0.0.1:8900".to_string()),
        Rc::new(payer_keypair),
        CommitmentConfig::confirmed(),
    );

    let program: Program<Rc<Keypair>> = client.program(program_id).unwrap();

    // Create the escrow first
    let make_ix = program
        .request()
        .accounts(anchor_escrow::client::accounts::Make {
            maker: maker.pubkey(),
            escrow: escrow_pda,
            mint_a: mint_a.pubkey(),
            mint_b: mint_b.pubkey(),
            maker_ata_a,
            vault,
            associated_token_program: spl_associated_token_account::id(),
            token_program: spl_token::id(),
            system_program: solana_sdk::system_program::id(),
        })
        .args(anchor_escrow::client::args::Make {
            seed,
            receive: 500_000_000,  // 0.5 tokens
            amount: 1_000_000_000,  // 1 token
        })
        .instructions()
        .unwrap()
        .remove(0);

    // Send make transaction
    let tx = Transaction::new_signed_with_payer(
        &[make_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("Escrow created successfully");

    // Now test the take instruction
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    let take_ix = program
        .request()
        .accounts(anchor_escrow::client::accounts::Take {
            taker: taker.pubkey(),
            maker: maker.pubkey(),
            escrow: escrow_pda,
            mint_a: mint_a.pubkey(),
            mint_b: mint_b.pubkey(),
            vault,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            associated_token_program: spl_associated_token_account::id(),
            token_program: spl_token::id(),
            system_program: solana_sdk::system_program::id(),
        })
        .args(anchor_escrow::client::args::Take {})
        .instructions()
        .unwrap()
        .remove(0);

    // Build and send take transaction using LiteSVM
    let tx = Transaction::new_signed_with_payer(
        &[take_ix],
        Some(&taker.pubkey()),
        &[&taker],
        svm.latest_blockhash(),
    );

    // Execute and verify
    let result = svm.send_transaction(tx);

    match result {
        Ok(res) => {
            println!("\nTake transaction succeeded with anchor_client!");
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

            println!("\nTake instruction test with anchor_client passed successfully!");
        }
        Err(e) => {
            panic!("Take transaction failed: {:?}", e);
        }
    }
}