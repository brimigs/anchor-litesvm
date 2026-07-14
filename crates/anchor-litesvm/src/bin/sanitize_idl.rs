//! Namespace an IDL's prelude-colliding type names so `declare_program!` can
//! ingest it (see `anchor_litesvm::idl_sanitize`). Used by `make fixtures` for
//! the staking IDL, which embeds mpl-core's `Key`.
//!
//! Usage: `sanitize_idl <in.json> [out.json]` (defaults to rewriting in place).

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .expect("usage: sanitize_idl <in.json> [out.json]");
    let output = args.next().unwrap_or_else(|| input.clone());

    let src = std::fs::read_to_string(&input).unwrap_or_else(|e| panic!("read {input}: {e}"));
    let sanitized =
        anchor_litesvm::sanitize_idl(&src).unwrap_or_else(|e| panic!("sanitize {input}: {e}"));
    std::fs::write(&output, sanitized).unwrap_or_else(|e| panic!("write {output}: {e}"));
    eprintln!("sanitized {input} -> {output}");
}
