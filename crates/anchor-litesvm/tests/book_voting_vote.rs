//! Book capture, TDD steps 3-5. The stage-3 client (whose IDL froze at
//! stage 3) drives both the naive stage-3 program and the guarded stage-4
//! program: the boundary did not change, only the behavior did.
#![allow(unexpected_cfgs)]
mod common;

use anchor_lang::{self};
use anchor_litesvm::{get_anchor_account, AnchorLiteSVM, Keypair, Signer};
use litesvm_utils::TestHelpers;

anchor_lang::declare_program!(voting_vote);
anchor_litesvm::bundles_from_idl!(voting_vote);

const POLL_ID: u64 = 1;

fn boot(stage_so: &str) -> anchor_litesvm::AnchorContext {
    AnchorLiteSVM::build_with_program(voting_vote::ID, "voting", &common::fixture_bytes(stage_so))
}

/// Create the poll and a "Cat" candidate as Alice, with the given window.
fn setup_poll(ctx: &mut anchor_litesvm::AnchorContext, alice: &Keypair, start: u64, end: u64) {
    let poll_account = common::voting::poll_pda(&voting_vote::ID, POLL_ID);
    ctx.tx(&[alice])
        .build(
            InitializePollBundle {
                signer: alice.pubkey(),
                poll_account,
            },
            voting_vote::client::args::InitializePoll {
                poll_id: POLL_ID,
                start,
                end,
                name: "Best Pet".to_string(),
                description: "Vote for the best pet".to_string(),
            },
        )
        .send_ok();
    let candidate_account = common::voting::candidate_pda(&voting_vote::ID, POLL_ID, "Cat");
    ctx.tx(&[alice])
        .build(
            InitializeCandidateBundle {
                signer: alice.pubkey(),
                poll_account,
                candidate_account,
            },
            voting_vote::client::args::InitializeCandidate {
                poll_id: POLL_ID,
                candidate: "Cat".to_string(),
            },
        )
        .send_ok();
}

fn vote_bundle(voter: &Keypair, candidate: &str) -> VoteBundle {
    VoteBundle {
        signer: voter.pubkey(),
        poll_account: common::voting::poll_pda(&voting_vote::ID, POLL_ID),
        candidate_account: common::voting::candidate_pda(&voting_vote::ID, POLL_ID, candidate),
        vote_receipt: common::voting::receipt_pda(&voting_vote::ID, POLL_ID, &voter.pubkey()),
    }
}

fn cast_vote_ix(
    ctx: &anchor_litesvm::AnchorContext,
    voter: &Keypair,
) -> solana_program::instruction::Instruction {
    ctx.program().build_ix(
        vote_bundle(voter, "Cat"),
        voting_vote::client::args::Vote {
            poll_id: POLL_ID,
            candidate: "Cat".to_string(),
        },
    )
}

#[test]
fn step3_vote_increments_the_tally() {
    let mut ctx = boot("voting_vote");
    let alice = ctx.cast_actor("Alice");
    setup_poll(&mut ctx, &alice, 1_000, 2_000);

    let bundle = vote_bundle(&alice, "Cat");
    let result = ctx
        .tx(&[&alice])
        .build(
            bundle,
            voting_vote::client::args::Vote {
                poll_id: POLL_ID,
                candidate: "Cat".to_string(),
            },
        )
        .send_ok();

    let candidate = common::voting::candidate_pda(&voting_vote::ID, POLL_ID, "Cat");
    let c: voting_vote::accounts::CandidateAccount =
        get_anchor_account(&ctx.svm, &candidate).expect("candidate exists");
    assert_eq!(c.candidate_votes, 1);

    common::expect_capture("voting_step3_green", &result.tree_string());
}

#[test]
fn step4_naive_program_accepts_an_out_of_window_vote() {
    // The naive stage-3 program has no time guard. We open a poll whose window
    // is entirely in the future, warp to before it, and vote. The forcing
    // question the test asks: "when exactly is voting allowed?" The naive
    // program has no answer, so the bad vote succeeds. That success IS the red.
    let mut ctx = boot("voting_vote");
    let alice = ctx.cast_actor("Alice");
    let now = ctx.svm.get_unix_timestamp();
    let start = (now + 10_000) as u64;
    let end = (now + 20_000) as u64;
    setup_poll(&mut ctx, &alice, start, end);

    // Clock is `now`, well before `start`: this vote should not be allowed.
    let bundle = vote_bundle(&alice, "Cat");
    let result = ctx
        .tx(&[&alice])
        .build(
            bundle,
            voting_vote::client::args::Vote {
                poll_id: POLL_ID,
                candidate: "Cat".to_string(),
            },
        )
        .send_ok();

    let candidate = common::voting::candidate_pda(&voting_vote::ID, POLL_ID, "Cat");
    let c: voting_vote::accounts::CandidateAccount =
        get_anchor_account(&ctx.svm, &candidate).expect("candidate exists");
    assert_eq!(
        c.candidate_votes, 1,
        "naive program let the early vote through"
    );

    common::expect_capture("voting_step4_red", &result.tree_string());
}

#[test]
fn step4_guarded_program_enforces_the_window() {
    // Same client, the guarded stage-4 program. The test codifies the exact
    // semantics the naive version left fuzzy: the window is `start < now <= end`.
    let mut ctx = boot("voting_guarded");
    let alice = ctx.cast_actor("Alice");
    let now = ctx.svm.get_unix_timestamp();
    let start = (now + 10_000) as u64;
    let end = (now + 20_000) as u64;
    setup_poll(&mut ctx, &alice, start, end);

    // Before start: VotingNotStarted.
    let ix = cast_vote_ix(&ctx, &alice);
    let result = ctx.send_err_named(ix, &[&alice], "VotingNotStarted");
    common::expect_capture("voting_step4_green", &result.tree_string());

    // Exactly at start is still closed (the boundary is exclusive).
    ctx.svm.warp_to_timestamp(start as i64);
    let ix = cast_vote_ix(&ctx, &alice);
    ctx.send_err_named(ix, &[&alice], "VotingNotStarted");

    // One second later it opens.
    ctx.svm.warp_to_timestamp(start as i64 + 1);
    let bundle = vote_bundle(&alice, "Cat");
    ctx.tx(&[&alice])
        .build(
            bundle,
            voting_vote::client::args::Vote {
                poll_id: POLL_ID,
                candidate: "Cat".to_string(),
            },
        )
        .send_ok();

    // After end: VotingEnded.
    ctx.svm.warp_to_timestamp(end as i64 + 1);
    let bob = ctx.cast_actor("Bob");
    let ix = cast_vote_ix(&ctx, &bob);
    ctx.send_err_named(ix, &[&bob], "VotingEnded");
}

#[test]
fn step5_double_vote_is_already_impossible() {
    // The forcing question: "what stops Mallory voting twice?" We write the
    // test expecting to add a guard, and it passes with no new code: the vote
    // receipt is seeded by poll + signer, so the second `init` collides on an
    // already-initialized account. The account model codified the invariant
    // for us; the test's job is to prove it.
    let mut ctx = boot("voting_guarded");
    let alice = ctx.cast_actor("Alice");
    let mallory = ctx.cast_actor("Mallory");
    let now = ctx.svm.get_unix_timestamp();
    setup_poll(&mut ctx, &alice, (now - 1) as u64, (now + 10_000) as u64);
    let poll_account = common::voting::poll_pda(&voting_vote::ID, POLL_ID);
    let dog_account = common::voting::candidate_pda(&voting_vote::ID, POLL_ID, "Dog");
    ctx.tx(&[&alice])
        .build(
            InitializeCandidateBundle {
                signer: alice.pubkey(),
                poll_account,
                candidate_account: dog_account,
            },
            voting_vote::client::args::InitializeCandidate {
                poll_id: POLL_ID,
                candidate: "Dog".to_string(),
            },
        )
        .send_ok();

    // Mallory votes once: fine.
    let bundle = vote_bundle(&mallory, "Cat");
    ctx.tx(&[&mallory])
        .build(
            bundle,
            voting_vote::client::args::Vote {
                poll_id: POLL_ID,
                candidate: "Cat".to_string(),
            },
        )
        .send_ok();

    // Mallory votes again, even for a different candidate: the receipt PDA
    // (poll + signer, no candidate) already exists, so `init` fails.
    let ix = ctx.program().build_ix(
        vote_bundle(&mallory, "Dog"),
        voting_vote::client::args::Vote {
            poll_id: POLL_ID,
            candidate: "Dog".to_string(),
        },
    );
    let result = ctx.send_err(ix, &[&mallory]);
    common::expect_capture("voting_step5_green", &result.tree_string());
}
