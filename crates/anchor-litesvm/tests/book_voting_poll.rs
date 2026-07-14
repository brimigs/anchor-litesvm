//! Book capture, TDD step 1: initialize_poll against the stage-1 program,
//! whose IDL knows only this one instruction. Snapshots the poll account's
//! state into book/src/captured/voting_step1_green.txt.
#![allow(unexpected_cfgs)]
mod common;

use anchor_lang::{self};
use anchor_litesvm::{get_anchor_account, AnchorLiteSVM, Signer};

anchor_lang::declare_program!(voting_poll);
anchor_litesvm::bundles_from_idl!(voting_poll);

fn boot() -> anchor_litesvm::AnchorContext {
    AnchorLiteSVM::build_with_program(
        voting_poll::ID,
        "voting",
        &common::fixture_bytes("voting_poll"),
    )
}

#[test]
fn step1_initialize_poll_creates_the_poll() {
    let mut ctx = boot();
    let alice = ctx.cast_actor("Alice");
    let poll_id = 1u64;

    // `poll_account`'s seeds reference `poll_id`, an instruction arg, so the
    // emitter can't derive it at build time and demotes it to a plain bundle
    // field: the caller derives and supplies it, same as escrow's `seed`-arg
    // PDA.
    let poll_account = common::voting::poll_pda(&voting_poll::ID, poll_id);

    let result = ctx
        .tx(&[&alice])
        .build(
            InitializePollBundle {
                signer: alice.pubkey(),
                poll_account,
            },
            voting_poll::client::args::InitializePoll {
                poll_id,
                start: 1_000,
                end: 2_000,
                name: "Best Pet".to_string(),
                description: "Vote for the best pet".to_string(),
            },
        )
        .send_ok();

    // The test forces the interface question ("what is a poll?") to be
    // answered before any logic: name, description, window, option counter.
    let acct: voting_poll::accounts::PollAccount =
        get_anchor_account(&ctx.svm, &poll_account).expect("poll account exists");
    assert_eq!(acct.poll_name, "Best Pet");
    assert_eq!(acct.poll_voting_start, 1_000);
    assert_eq!(acct.poll_voting_end, 2_000);
    assert_eq!(acct.poll_option_index, 0);

    common::expect_capture("voting_step1_green", &result.tree_string());
}
