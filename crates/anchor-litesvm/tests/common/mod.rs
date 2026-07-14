//! Shared helpers for the book capture tests.
//!
//! `expect_capture` is a golden-file assertion: it pins a test's rendered
//! output to a committed snapshot under `book/src/captured/`. The book
//! `{{#include}}`s those snapshots, so the docs are literally the test output.
//! `BLESS=1 cargo test` rewrites them; a plain run fails on any drift.
use std::path::PathBuf;

fn captured_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../book/src/captured")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(format!("{name}.so"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

/// Normalize to trailing-newline, no trailing-whitespace, single final newline
/// so cosmetic differences never trip the golden compare.
fn normalize(s: &str) -> String {
    let mut out: String = s.lines().map(|l| format!("{}\n", l.trim_end())).collect();
    if out.is_empty() {
        out.push('\n');
    }
    out
}

pub fn expect_capture(name: &str, actual: &str) {
    let path = captured_dir().join(format!("{name}.txt"));
    let actual = normalize(actual);
    if std::env::var("BLESS").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual.as_bytes()).unwrap();
        eprintln!("blessed {}", path.display());
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run `BLESS=1 cargo test -p anchor-litesvm` to create it",
            path.display()
        )
    });
    assert_eq!(
        normalize(&expected),
        actual,
        "snapshot drift for `{name}`; run `BLESS=1 cargo test -p anchor-litesvm` to update"
    );
}

/// PDA derivations for the voting tutorial, matching the program's seeds
/// verbatim. Every stage shares the same program id, so the id is a
/// parameter rather than a per-stage constant.
pub mod voting {
    use solana_program::pubkey::Pubkey;

    pub fn poll_pda(program_id: &Pubkey, poll_id: u64) -> Pubkey {
        Pubkey::find_program_address(&[b"poll", &poll_id.to_le_bytes()], program_id).0
    }

    pub fn candidate_pda(program_id: &Pubkey, poll_id: u64, candidate: &str) -> Pubkey {
        Pubkey::find_program_address(&[&poll_id.to_le_bytes(), candidate.as_bytes()], program_id).0
    }

    pub fn receipt_pda(program_id: &Pubkey, poll_id: u64, voter: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"vote_receipt", &poll_id.to_le_bytes(), voter.as_ref()],
            program_id,
        )
        .0
    }
}
