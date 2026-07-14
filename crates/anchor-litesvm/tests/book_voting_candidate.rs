//! Book capture, TDD step 2: initialize_candidate against the stage-2
//! program. Its IDL has grown by exactly one instruction.
#![allow(unexpected_cfgs)]
mod common;

use anchor_lang::{self};
use anchor_litesvm::{get_anchor_account, AnchorLiteSVM, Signer};

anchor_lang::declare_program!(voting_candidate);
anchor_litesvm::bundles_from_idl!(voting_candidate);

fn boot() -> anchor_litesvm::AnchorContext {
    AnchorLiteSVM::build_with_program(
        voting_candidate::ID,
        "voting",
        &common::fixture_bytes("voting_candidate"),
    )
}

#[test]
fn step2_initialize_candidate_registers_under_a_poll() {
    let mut ctx = boot();
    let alice = ctx.cast_actor("Alice");
    let poll_id = 1u64;

    let poll_account = common::voting::poll_pda(&voting_candidate::ID, poll_id);

    ctx.tx(&[&alice])
        .build(
            InitializePollBundle {
                signer: alice.pubkey(),
                poll_account,
            },
            voting_candidate::client::args::InitializePoll {
                poll_id,
                start: 1_000,
                end: 2_000,
                name: "Best Pet".to_string(),
                description: "Vote for the best pet".to_string(),
            },
        )
        .send_ok();

    let candidate_account = common::voting::candidate_pda(&voting_candidate::ID, poll_id, "Cat");

    let result = ctx
        .tx(&[&alice])
        .build(
            InitializeCandidateBundle {
                signer: alice.pubkey(),
                poll_account,
                candidate_account,
            },
            voting_candidate::client::args::InitializeCandidate {
                poll_id,
                candidate: "Cat".to_string(),
            },
        )
        .send_ok();

    let c: voting_candidate::accounts::CandidateAccount =
        get_anchor_account(&ctx.svm, &candidate_account).expect("candidate exists");
    assert_eq!(c.candidate_name, "Cat");
    assert_eq!(c.candidate_votes, 0);

    let p: voting_candidate::accounts::PollAccount =
        get_anchor_account(&ctx.svm, &poll_account).expect("poll exists");
    assert_eq!(p.poll_option_index, 1);

    common::expect_capture("voting_step2_green", &result.tree_string());
}
