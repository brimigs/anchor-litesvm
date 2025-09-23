use anchor_litesvm::AnchorLiteSVM;
use litesvm_utils::{AssertionHelpers, TestHelpers};
use solana_sdk::signature::Signer;
use spl_associated_token_account::get_associated_token_address;

// Generate client modules from IDL
anchor_lang::declare_program!(anchor_escrow);

#[test]
fn test_escrow_with_anchor_litesvm() {
    // 1-line initialization with anchor-client integration!
    let mut ctx = AnchorLiteSVM::build_with_program(
        anchor_escrow::ID,
        include_bytes!("../../target/deploy/anchor_escrow.so"),
    );

    // Create test accounts - now we can use ctx.svm directly!
    let maker = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    // Create tokens using litesvm-utils methods directly on ctx.svm
    let mint_a = ctx.svm.create_token_mint(&maker, 9).unwrap();
    let mint_b = ctx.svm.create_token_mint(&maker, 9).unwrap();

    // Create and fund associated token accounts
    let maker_ata_a = ctx.svm.create_associated_token_account(&mint_a.pubkey(), &maker).unwrap();
    ctx.svm.mint_to(&mint_a.pubkey(), &maker_ata_a, &maker, 1_000_000_000).unwrap();
    let taker_ata_b = ctx.svm.create_associated_token_account(&mint_b.pubkey(), &taker).unwrap();
    ctx.svm.mint_to(&mint_b.pubkey(), &taker_ata_b, &maker, 500_000_000).unwrap();

    // Calculate PDAs using the new helper
    let seed: u64 = 42;
    let escrow_pda = ctx.svm.get_pda(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &anchor_escrow::ID,
    );
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // MAKE: Build instruction using native builder (no RPC overhead!)
    let make_ix = ctx.instruction()
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
        .build();

    // Execute using the convenience method
    ctx.execute_instruction(make_ix, &[&maker])
        .unwrap()
        .assert_success();

    // Verify make operation - direct access to litesvm-utils assertions!
    assert!(ctx.account_exists(&escrow_pda));
    ctx.svm.assert_token_balance(&vault, 1_000_000_000);
    ctx.svm.assert_token_balance(&maker_ata_a, 0);

    // TAKE: Build instruction using native builder
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    let take_ix = ctx.instruction()
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
        .build();

    // Execute the take instruction
    ctx.execute_instruction(take_ix, &[&taker])
        .unwrap()
        .assert_success();

    // Final verification - clean and direct!
    ctx.svm.assert_account_closed(&escrow_pda);
    ctx.svm.assert_account_closed(&vault);
    ctx.svm.assert_token_balance(&taker_ata_a, 1_000_000_000);
    ctx.svm.assert_token_balance(&maker_ata_b, 500_000_000);
}