//! TDD tutorial interface reds, drift-checked at the boundary: each stage's
//! IDL must list exactly the instructions declared so far. If a stage's
//! instruction set drifts, this fails. The compiler-error form of the red
//! (the typed client lacking the instruction) is shown illustratively in the
//! chapter; it can't be a trybuild case because `declare_program!` resolves
//! `idls/` relative to `CARGO_MANIFEST_DIR`, which trybuild relocates to a
//! temp dir without `idls/`.
use std::path::PathBuf;

fn idl_path(stage: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("idls")
        .join(format!("{stage}.json"))
}

fn instruction_names(stage: &str) -> Vec<String> {
    let path = idl_path(stage);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid IDL JSON");
    json["instructions"]
        .as_array()
        .expect("instructions array")
        .iter()
        .map(|i| i["name"].as_str().expect("instruction name").to_string())
        .collect()
}

#[test]
fn idl_instruction_set_grows_then_freezes() {
    assert_eq!(
        instruction_names("voting_empty"),
        Vec::<String>::new(),
        "stage0 is empty"
    );

    assert_eq!(instruction_names("voting_poll"), ["initialize_poll"]);
    // Interface red for step 2: stage 1 does not declare initialize_candidate.
    assert!(!instruction_names("voting_poll").contains(&"initialize_candidate".to_string()));

    let s2 = instruction_names("voting_candidate");
    assert!(s2.contains(&"initialize_poll".to_string()));
    assert!(s2.contains(&"initialize_candidate".to_string()));
    // Interface red for step 3: stage 2 does not declare vote.
    assert!(!s2.contains(&"vote".to_string()));

    let mut s3 = instruction_names("voting_vote");
    s3.sort();
    assert_eq!(s3, ["initialize_candidate", "initialize_poll", "vote"]);

    // The IDL froze: stage 3 and stage 4 are byte-identical (a guard is
    // behavior, not interface).
    assert_eq!(
        std::fs::read(idl_path("voting_vote")).unwrap(),
        std::fs::read(idl_path("voting_guarded")).unwrap(),
        "stage3 == stage4"
    );
}
