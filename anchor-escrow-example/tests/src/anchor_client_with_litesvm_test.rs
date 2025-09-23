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
fn test_escrow_with_anchor_client_and_litesvm() {
    // ============ SETUP PHASE ============

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
        1_000_000_000, // 1 token
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Create taker's associated token account for mint_b
    let taker_ata_b = get_associated_token_address(&taker.pubkey(), &mint_b.pubkey());
    let create_taker_ata_b_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &taker.pubkey(),
        &taker.pubkey(),
        &mint_b.pubkey(),
        &spl_token::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_taker_ata_b_ix],
        Some(&taker.pubkey()),
        &[&taker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Mint tokens to taker's ATA for mint_b
    let mint_to_taker_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_b.pubkey(),
        &taker_ata_b,
        &maker.pubkey(),  // maker is mint authority
        &[],
        500_000_000,  // 0.5 tokens
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_taker_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // ============ ANCHOR CLIENT SETUP ============

    // Set up anchor-client with mock RPC (required for anchor-client)
    let payer = Rc::new(maker.clone());
    let _mock_rpc = RpcClient::new_mock("succeeds".to_string());

    // Create anchor-client instance with mock cluster
    let client = Client::new_with_options(
        Cluster::Custom(
            "http://127.0.0.1:8899".to_string(),
            "ws://127.0.0.1:8900".to_string(),
        ),
        payer.clone(),
        CommitmentConfig::confirmed(),
    );

    let program: Program<Rc<Keypair>> = client.program(program_id).unwrap();

    // ============ MAKE INSTRUCTION ============

    // Calculate PDA
    let seed = 42u64;
    let escrow_pda = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), seed.to_le_bytes().as_ref()],
        &anchor_escrow::ID,
    ).0;

    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // Build MAKE instruction using anchor-client
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

    // Execute MAKE transaction with LiteSVM
    let tx = Transaction::new_signed_with_payer(
        &[make_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx).unwrap();

    // Verify MAKE succeeded
    assert!(result.is_ok(), "Make transaction failed");

    // Verify escrow was created
    assert!(svm.get_account(&escrow_pda).is_some(), "Escrow account not created");

    // Verify tokens were transferred to vault
    let vault_account = svm.get_account(&vault).unwrap();
    let vault_token_account = spl_token::state::Account::unpack(&vault_account.data).unwrap();
    assert_eq!(vault_token_account.amount, 1_000_000_000, "Vault doesn't have correct amount");

    // ============ TAKE INSTRUCTION ============

    // Create required ATAs for TAKE
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    // Build TAKE instruction using anchor-client
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

    // Execute TAKE transaction with LiteSVM
    let tx = Transaction::new_signed_with_payer(
        &[take_ix],
        Some(&taker.pubkey()),
        &[&taker],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx).unwrap();

    // Verify TAKE succeeded
    assert!(result.is_ok(), "Take transaction failed");

    // Verify escrow was closed
    assert!(svm.get_account(&escrow_pda).is_none(), "Escrow account not closed");

    // Verify final token balances
    let taker_ata_a_account = svm.get_account(&taker_ata_a).unwrap();
    let taker_ata_a_token = spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
    assert_eq!(taker_ata_a_token.amount, 1_000_000_000, "Taker didn't receive mint A tokens");

    let maker_ata_b_account = svm.get_account(&maker_ata_b).unwrap();
    let maker_ata_b_token = spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
    assert_eq!(maker_ata_b_token.amount, 500_000_000, "Maker didn't receive mint B tokens");
}