use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, Signer, TestHelpers};
use solana_program::pubkey::Pubkey;

fn main() {
    println!("anchor-litesvm advanced_features");
    println!("This packaged example is compile-oriented and highlights PDA, token, and assertion helpers.");
    println!("See the source for a minimal example that exercises the public API surface.");
}

#[allow(dead_code)]
fn compile_only_example() {
    let program_id = Pubkey::new_unique();
    let mut ctx = AnchorLiteSVM::build_with_program(program_id, "program", &[]);
    let authority = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&authority, 9).unwrap();
    let user_ata = ctx
        .svm
        .create_associated_token_account(&mint.pubkey(), &user)
        .unwrap();
    let (vault, _bump) = ctx
        .svm
        .get_pda_with_bump(&[b"vault", user.pubkey().as_ref()], &program_id);

    ctx.svm
        .mint_to(&mint.pubkey(), &user_ata, &authority, 500_000)
        .unwrap();

    ctx.svm.assert_token_balance(&user_ata, 500_000);
    ctx.svm.assert_account_closed(&vault);
}
