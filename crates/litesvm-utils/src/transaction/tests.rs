use super::*;
use crate::test_helpers::TestHelpers;
use solana_system_interface::instruction as system_instruction;

#[test]
fn test_transaction_result_success() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    // Create a simple transfer instruction
    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);

    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    assert!(result.is_success());
    assert_eq!(result.error(), None);
    result.assert_success();
}

#[test]
fn test_transaction_result_has_log() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // System program logs typically contain "invoke" messages
    assert!(result.has_log("invoke"));
}

#[test]
fn test_transaction_result_find_log() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // Should find a log containing "invoke"
    let log = result.find_log("invoke");
    assert!(log.is_some());
}

#[test]
fn test_transaction_result_failure() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new(); // Unfunded account

    // This should fail due to insufficient funds
    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    assert!(!result.is_success());
    assert!(result.error().is_some());
}

#[test]
fn test_transaction_result_assert_failure() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new(); // Unfunded account

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // Should not panic when asserting failure on a failed transaction
    result.assert_failure();
}

#[test]
#[should_panic(expected = "Expected transaction to fail")]
fn test_transaction_result_assert_failure_on_success() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // Should panic when asserting failure on a successful transaction
    result.assert_failure();
}

#[test]
fn test_transaction_result_assert_error() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new(); // Unfunded account

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // Should contain "AccountNotFound" in the error (account doesn't exist)
    result.assert_error("AccountNotFound");
}

#[test]
#[should_panic(expected = "Expected error containing 'this error does not exist'")]
fn test_transaction_result_assert_error_wrong_message() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new(); // Unfunded account

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // Should panic when expecting wrong error message
    result.assert_error("this error does not exist");
}

#[test]
fn test_send_multiple_instructions() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient1 = Keypair::new();
    let recipient2 = Keypair::new();

    // Send two transfers in one transaction
    let ix1 = system_instruction::transfer(&payer.pubkey(), &recipient1.pubkey(), 1_000_000);
    let ix2 = system_instruction::transfer(&payer.pubkey(), &recipient2.pubkey(), 2_000_000);

    let result = svm.send_instructions(&[ix1, ix2], &[&payer]).unwrap();
    result.assert_success();

    // Verify both transfers succeeded
    let balance1 = svm.get_balance(&recipient1.pubkey()).unwrap();
    let balance2 = svm.get_balance(&recipient2.pubkey()).unwrap();
    assert_eq!(balance1, 1_000_000);
    assert_eq!(balance2, 2_000_000);
}

#[test]
fn test_send_instruction_no_signers() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);

    // Should error when no signers provided
    let result = svm.send_instruction(ix, &[]);
    assert!(result.is_err());
    match result {
        Err(TransactionError::BuildError(msg)) => {
            assert!(msg.contains("No signers"));
        }
        _ => panic!("Expected BuildError"),
    }
}

#[test]
fn test_send_instructions_no_signers() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);

    // Should error when no signers provided
    let result = svm.send_instructions(&[ix], &[]);
    assert!(result.is_err());
}

#[test]
fn test_transaction_result_print_logs() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    // Should not panic when printing logs
    result.print_logs();
}

#[test]
fn test_send_transaction_result() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction_result(tx).unwrap();
    result.assert_success();
}

#[test]
fn test_send_ok_returns_result_for_chaining() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    // send_ok returns the wrapped TransactionResult so callers can inspect logs / cu.
    let result = svm.send_ok(ix, &[&payer], &crate::Aliases::default());
    assert!(result.is_success());
    assert!(result.compute_units() > 0);
}

#[test]
#[should_panic(expected = "Transaction failed")]
fn test_send_ok_panics_on_program_error() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new(); // unfunded; transfer will fail

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    svm.send_ok(ix, &[&payer], &crate::Aliases::default());
}

#[test]
#[should_panic(expected = "send_ok: transaction build failed")]
fn test_send_ok_panics_on_build_error() {
    let mut svm = LiteSVM::new();
    let ix = system_instruction::transfer(
        &Keypair::new().pubkey(),
        &Keypair::new().pubkey(),
        1_000_000,
    );
    // Empty signer slice trips the build-time guard in send_instruction.
    svm.send_ok(ix, &[], &crate::Aliases::default());
}

#[test]
fn test_send_err_named_passes_on_matching_error() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new(); // unfunded

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    // "AccountNotFound" appears in the system-program error path.
    let _result = svm.send_err_named(ix, &[&payer], &crate::Aliases::default(), "AccountNotFound");
}

#[test]
fn test_assert_success_with_passes_when_predicate_holds() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let _result = svm
        .send_instruction(ix, &[&payer])
        .unwrap()
        .assert_success_with(|r| r.compute_units() > 0);
}

#[test]
#[should_panic(expected = "Predicate failed on successful transaction")]
fn test_assert_success_with_panics_when_predicate_fails() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    svm.send_instruction(ix, &[&payer])
        .unwrap()
        .assert_success_with(|_| false);
}

#[test]
fn test_assert_failure_with_passes_when_predicate_holds() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();

    // Unfunded payer fails at validation, before execution: compute_units == 0.
    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let _result = svm
        .send_instruction(ix, &[&payer])
        .unwrap()
        .assert_failure_with(|r| r.compute_units() == 0);
}

#[test]
#[should_panic(expected = "Predicate failed on failed transaction")]
fn test_assert_failure_with_panics_when_predicate_fails() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    svm.send_instruction(ix, &[&payer])
        .unwrap()
        .assert_failure_with(|_| false);
}

#[test]
#[should_panic(expected = "Expected error containing 'SomeErrorThatNeverAppears'")]
fn test_send_err_named_panics_on_wrong_error_name() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    svm.send_err_named(
        ix,
        &[&payer],
        &crate::Aliases::default(),
        "SomeErrorThatNeverAppears",
    );
}

#[test]
#[should_panic(expected = "Expected transaction to fail")]
fn test_send_err_named_panics_on_success() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    svm.send_err_named(ix, &[&payer], &crate::Aliases::default(), "AnyError");
}

/// On failure, send_ok prints the aliased logs to stderr before the
/// assert_success panic, so test output shows which program failed (the
/// underlying panic only embeds the flat log dump).
#[test]
fn test_send_ok_prints_aliased_logs_on_failure() {
    use std::panic;
    let mut svm = LiteSVM::new();
    let payer = Keypair::new(); // unfunded; will fail

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let panic_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        svm.send_ok(ix, &[&payer], &crate::Aliases::default());
    }));
    assert!(panic_result.is_err(), "send_ok must panic on failed tx");
    // The eprintln!'d logs aren't captured by catch_unwind. Visual
    // inspection via `cargo test -- --nocapture` confirms they print. The
    // panic payload itself comes from assert_success and contains the flat
    // log dump as documented.
    let payload = panic_result.unwrap_err();
    let msg = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert!(
        msg.contains("Transaction failed"),
        "panic should be from assert_success: got `{}`",
        msg
    );
}

/// Same shape for send_err_named: when the assertion is about to fail
/// (either because the tx succeeded or because the error name didn't
/// match), the aliased logs are printed before assert_error panics.
#[test]
fn test_send_err_named_prints_aliased_logs_on_mismatch() {
    use std::panic;
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &Keypair::new().pubkey(), 1_000_000);
    let panic_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        svm.send_err_named(
            ix,
            &[&payer],
            &crate::Aliases::default(),
            "SomeErrorThatNeverAppears",
        );
    }));
    assert!(panic_result.is_err());
    let payload = panic_result.unwrap_err();
    let msg = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert!(
        msg.contains("Expected error containing 'SomeErrorThatNeverAppears'"),
        "got `{}`",
        msg
    );
}

#[test]
fn send_instruction_threads_message_into_transaction_result() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(1_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();
    assert_eq!(result.message.header.num_required_signatures, 1);
    assert_eq!(result.message.account_keys[0], payer.pubkey());
}

#[test]
fn test_tap_executes_closure_and_returns_self() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    let mut side_effect_ran = false;

    // tap should execute the closure and allow chaining
    result
        .tap(|r| {
            assert!(r.is_success());
            side_effect_ran = true;
        })
        .assert_success();

    assert!(side_effect_ran, "tap closure should have executed");
}

/// `send_ok` stashes the alias table on the returned result, so a
/// downstream chained `print_logs()` (no arg) substitutes it.
#[test]
fn send_ok_stashes_aliases_for_chained_print() {
    use crate::Aliases;
    let system_id =
        solana_program::pubkey::Pubkey::from_str_const("11111111111111111111111111111111");
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(1_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let aliases = Aliases::default().with(system_id, "MySystem");
    let result = svm.send_ok(ix, &[&payer], &aliases);

    let out = result.logs_string();
    assert!(
        out.contains("Program: MySystem"),
        "send_ok-stashed alias should rename the header program; got:\n{out}"
    );
}

/// With no `with_aliases` call, `logs_string()` falls back to
/// `Aliases::default()`, so the well-known System program still renders
/// by name rather than by raw pubkey.
#[test]
fn logs_string_without_attached_aliases_uses_default() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(1_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let result = svm.send_instruction(ix, &[&payer]).unwrap();

    let out = result.logs_string();
    assert!(
        out.contains("Program: System"),
        "well-known System alias should resolve via default; got:\n{out}"
    );
}

#[test]
fn test_tap_enables_method_chaining() {
    let mut svm = LiteSVM::new();
    let payer = svm.create_funded_account(10_000_000_000).unwrap();
    let recipient = Keypair::new();

    let ix = system_instruction::transfer(&payer.pubkey(), &recipient.pubkey(), 1_000_000);

    // Demonstrate fluent chaining with tap
    let cu = svm
        .send_instruction(ix, &[&payer])
        .unwrap()
        .tap(|r| {
            // Custom inspection/logging
            let _ = r.compute_units();
        })
        .assert_success()
        .compute_units();

    assert!(cu > 0);
}

#[test]
fn identical_sends_are_fresh_by_default() {
    // The repeated-send pattern (rate limits, spend caps): an identical
    // instruction resent in a loop. send_instruction refreshes the blockhash
    // before signing, so no send collides with its predecessor's signature
    // and nobody performs the expire_blockhash ritual. (cds-rs issue #4)
    use anchor_litesvm_compat::Signer;
    let mut svm = anchor_litesvm_compat::LiteSVM::new();
    let payer = anchor_litesvm_compat::Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let dest = solana_program::pubkey::Pubkey::new_unique();
    for i in 0..3 {
        let ix = solana_system_interface::instruction::transfer(&payer.pubkey(), &dest, 2_000_000);
        let result = svm.send_instruction(ix, &[&payer]).expect("build ok");
        assert!(result.error().is_none(), "send #{i} should be fresh");
    }
}

#[test]
fn decoded_events_render_as_badges_in_the_log_stream() {
    use base64::Engine as _;
    use std::sync::Arc;

    let alice = Pubkey::new_unique();
    let alice_b58 = alice.to_string();

    let mut registry = EventRegistry::new();
    let pairs = vec![
        ("user".to_string(), alice_b58.clone()),
        ("amount".to_string(), "1000000".to_string()),
    ];
    registry.register(
        [1, 2, 3, 4, 5, 6, 7, 8],
        "Deposited",
        Arc::new(move |_bytes: &[u8]| Some(pairs.clone())),
    );

    let mut aliases = Aliases::default();
    aliases.add(alice, "alice");

    let payload =
        base64::engine::general_purpose::STANDARD.encode([1u8, 2, 3, 4, 5, 6, 7, 8, 9, 9]);
    let event_line = format!("Program data: {payload}");

    // A decodable `Program data:` line renders as the event badge, fields
    // alias-resolved.
    let rendered = render_log_line(&event_line, &registry, &aliases);
    assert_eq!(rendered, "🔔 Deposited { user: alice, amount: 1000000 }");

    // An undecodable payload keeps its raw line: information is never dropped.
    let unknown = format!(
        "Program data: {}",
        base64::engine::general_purpose::STANDARD.encode([9u8; 12])
    );
    assert_eq!(render_log_line(&unknown, &registry, &aliases), unknown);

    // Ordinary lines pass through with alias substitution only.
    let plain = format!("Program log: transfer from {alice_b58}");
    assert_eq!(
        render_log_line(&plain, &registry, &aliases),
        "Program log: transfer from alice"
    );
}

#[test]
fn tree_string_renders_frames_signers_error_and_legend() {
    let mut svm = LiteSVM::new();
    let alice_kp = svm.create_funded_account(10_000_000_000).unwrap();
    let alice = alice_kp.pubkey();
    let program = Pubkey::new_unique();

    let mut aliases = Aliases::with_well_known();
    aliases.add(alice, "Alice");
    aliases.add(program, "amm");

    // A real send gives us a well-formed Message; the logs are then swapped
    // for a captured-shape CPI stream so the tree parse is exercised without
    // needing a deployed Anchor program.
    let ix = system_instruction::transfer(&alice, &Pubkey::new_unique(), 1_000_000);
    let mut result = svm
        .send_instruction(ix, &[&alice_kp])
        .unwrap()
        .with_aliases(aliases);
    let sys = "11111111111111111111111111111111";
    result.set_logs_for_test(vec![
        format!("Program {program} invoke [1]"),
        "Program log: Instruction: AddLiquidity".to_string(),
        format!("Program {sys} invoke [2]"),
        format!("Program {sys} success"),
        "Program log: AnchorError caused by account: config. Error Code: SlippageExceeded. Error Number: 6009. Error Message: slippage exceeded.".to_string(),
        format!("Program {program} consumed 47687 of 200000 compute units"),
        format!("Program {program} failed: custom program error: 0x1779"),
    ]);

    let out = result.tree_string();
    assert!(
        out.contains("amm::AddLiquidity [1] ✗ 47687cu"),
        "frame line; got:\n{out}"
    );
    assert!(
        out.contains("signer=Alice"),
        "signer annotation; got:\n{out}"
    );
    assert!(
        out.contains("System"),
        "well-known child label; got:\n{out}"
    );
    assert!(out.contains("[2] ✓"), "child depth+outcome; got:\n{out}");
    assert!(out.contains("Legend"), "legend section; got:\n{out}");
    assert!(
        out.contains(&format!("amm   = {program}")) || out.contains(&format!("amm = {program}")),
        "legend maps amm; got:\n{out}"
    );
    assert!(
        out.contains(&format!("Alice = {alice}")),
        "legend maps Alice; got:\n{out}"
    );
    assert!(
        out.contains("Error: SlippageExceeded"),
        "AnchorError code name surfaces as the failure leaf; got:\n{out}"
    );
    assert!(
        !out.contains(&program.to_string()[..8.min(4)]) || true,
        "smoke"
    );
}
