//! Anchor IDL parsing layer for the `bundles_from_idl!` proc-macro.
//!
//! This module extracts the minimal subset of an Anchor IDL the macro consumes to build bundle
//! fixtures: the program address, instruction names, account names and properties (optional,
//! hard-coded addresses), and PDA descriptors (seed const values and account paths, plus the
//! foreign derivation program an associated-token account carries). Everything
//! else a bundle needs (argument types, instruction discriminators, account signer/writable flags)
//! comes from the `declare_program!`-generated `client` module, so this layer does not model it;
//! those keys, when present in the IDL JSON, are ignored.

use serde::Deserialize;

/// The top-level IDL: program address and instructions.
#[derive(Deserialize, Debug, Clone)]
pub struct Idl {
    pub address: String,
    pub instructions: Vec<IdlInstruction>,
}

/// An instruction: its name and account list.
#[derive(Deserialize, Debug, Clone)]
pub struct IdlInstruction {
    pub name: String,
    pub accounts: Vec<IdlAccount>,
}

/// An account in an instruction's account list.
///
/// `deny_unknown_fields` closes the same class of bug `IdlPda` guards
/// against, one level up. `docs`, `relations`, `writable`, and `signer` are
/// real Anchor IDL keys (verified against a corpus of Anchor 0.31 IDLs —
/// this crate's own fixtures, `brimigs-anchor-escrow`, the `quarry_*`
/// programs, and an AMM capstone program — and cross-checked against
/// `anchor-lang-idl-spec`'s `IdlInstructionAccount`) that this layer has no
/// build-time use for (the `declare_program!`-generated `client::accounts`
/// struct already carries writable/signer, and prose/relation hints don't
/// bear on account addresses), so they're modeled as parsed-but-unused
/// rather than left to fall through a permissive struct. If Anchor ever adds
/// a semantics-bearing account key (as it did with `pda.program`), this now
/// errors naming it instead of dropping it silently.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct IdlAccount {
    pub name: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub pda: Option<IdlPda>,
    // Prose only, no build semantics.
    #[serde(default)]
    #[allow(dead_code)]
    pub docs: Vec<String>,
    // Anchor client-side account resolution (fills accounts in from other
    // accounts' relations); no address semantics, so unused here.
    #[serde(default)]
    #[allow(dead_code)]
    pub relations: Vec<String>,
    // `declare_program!`'s generated `client::accounts` struct already
    // carries writable/signer flags; the macro never reads them off the IDL.
    #[serde(default)]
    #[allow(dead_code)]
    pub writable: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub signer: bool,
}

/// A PDA descriptor for an account: the seeds, plus (for a foreign-program PDA
/// like an associated-token account) the program the derivation runs under. When
/// `program` is absent the PDA derives under the IDL's own program.
///
/// `deny_unknown_fields` is the guard that keeps this bug closed: Anchor emitted
/// `program` here for every `associated_token::` account long before this struct
/// modeled it, and serde silently dropped it, so ATAs derived under the wrong
/// program with no diagnostic. Any future PDA key we don't model now fails
/// parsing loudly rather than being quietly ignored.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct IdlPda {
    pub seeds: Vec<IdlSeed>,
    #[serde(default)]
    pub program: Option<IdlSeed>,
}

/// A single seed component in a PDA derivation (reused for the `program` of a
/// foreign-program PDA). `Const` carries the raw bytes; `Account` a path to
/// another account (with an optional type name Anchor emits for typed-data
/// paths, which the macro ignores); `Arg` a path to an instruction argument the
/// macro can't resolve to a pubkey at build time (so such accounts demote to a
/// bundle field). `deny_unknown_fields` keeps a future seed key from being
/// dropped silently.
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum IdlSeed {
    Const {
        value: Vec<u8>,
    },
    Account {
        path: String,
        // Modeled only so `deny_unknown_fields` accepts the real IDL shape
        // (Anchor emits a type name for typed-data seed paths); the macro derives
        // from the path alone and never reads the type.
        #[serde(default)]
        #[allow(dead_code)]
        account: Option<String>,
    },
    Arg {
        // Modeled only to satisfy `deny_unknown_fields`; an arg seed's path can't
        // resolve to a pubkey at build time, so the account demotes to a field
        // and the path is never read.
        #[serde(default)]
        #[allow(dead_code)]
        path: Option<String>,
    },
}

impl Idl {
    /// Parse an Anchor IDL JSON string into the `Idl` struct.
    ///
    /// # Errors
    ///
    /// Returns a stringified JSON error if deserialization fails.
    /// Unknown seed kinds (other than `const`, `account`, `arg`) produce an error and do not
    /// silently degrade.
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("failed to parse IDL: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> Idl {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/idls/vault.json"
        ))
        .unwrap();
        Idl::parse(&json).unwrap()
    }

    #[test]
    fn parses_vault_shape() {
        let idl = vault();
        assert_eq!(idl.address, "6RviLVy2WPGm7QYfCuZq66vKWF58WVTNWfFE7RgWxcfP");
        let close = idl.instructions.iter().find(|i| i.name == "close").unwrap();
        let sys = close
            .accounts
            .iter()
            .find(|a| a.name == "system_program")
            .unwrap();
        assert_eq!(
            sys.address.as_deref(),
            Some("11111111111111111111111111111111")
        );
        let vault_acc = close.accounts.iter().find(|a| a.name == "vault").unwrap();
        let seeds = &vault_acc.pda.as_ref().unwrap().seeds;
        assert!(matches!(&seeds[0], IdlSeed::Const { value } if value == b"vault"));
        assert!(matches!(&seeds[1], IdlSeed::Account { path, .. } if path == "vault_state"));
    }

    #[test]
    fn arg_seed_parses_with_path() {
        // A real IDL carries the arg seed's path; the macro can't derive a PDA
        // from an instruction arg, but the key is modeled so `deny_unknown_fields`
        // doesn't reject the real shape.
        let seed: IdlSeed = serde_json::from_str(r#"{"kind":"arg","path":"seed"}"#).unwrap();
        assert!(matches!(seed, IdlSeed::Arg { path } if path.as_deref() == Some("seed")));
    }

    #[test]
    fn account_seed_parses_with_type_name() {
        // Anchor emits an `account` type name alongside the path for typed-data
        // seed paths; the macro ignores it but must not reject it.
        let seed: IdlSeed =
            serde_json::from_str(r#"{"kind":"account","path":"auth.admin","account":"OnlyAdmin"}"#)
                .unwrap();
        assert!(matches!(seed, IdlSeed::Account { path, account }
            if path == "auth.admin" && account.as_deref() == Some("OnlyAdmin")));
    }

    #[test]
    fn pda_program_const_parses() {
        // The `program` key an `associated_token::` account carries: 32 raw bytes
        // of the program the PDA derives under.
        let pda: IdlPda = serde_json::from_str(
            r#"{"seeds":[{"kind":"account","path":"user"}],"program":{"kind":"const","value":[1,2,3]}}"#,
        )
        .unwrap();
        assert!(matches!(pda.program, Some(IdlSeed::Const { value }) if value == [1, 2, 3]));
    }

    #[test]
    fn seed_unknown_field_errors() {
        // The same loud-failure guard applies to seeds: an unmodeled seed key
        // fails parsing rather than being dropped.
        let err = serde_json::from_str::<IdlSeed>(r#"{"kind":"const","value":[1],"bogus":9}"#)
            .unwrap_err();
        assert!(
            err.to_string().contains("bogus"),
            "error must name the unknown seed key: {err}"
        );
    }

    #[test]
    fn pda_unknown_field_errors() {
        // A future PDA key we don't model must fail parsing loudly, naming the
        // key, rather than being silently dropped (the bug this closes).
        let err = serde_json::from_str::<IdlPda>(
            r#"{"seeds":[{"kind":"account","path":"user"}],"programz":{"kind":"const","value":[1]}}"#,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("programz"),
            "error must name the unknown key: {err}"
        );
    }

    #[test]
    fn account_unknown_field_errors() {
        // The same class of bug `pda.program` was, one level up: a future
        // account-level key must fail parsing loudly, naming the key, rather
        // than being silently dropped.
        let err =
            serde_json::from_str::<IdlAccount>(r#"{"name":"user","bogus":true}"#).unwrap_err();
        assert!(
            err.to_string().contains("bogus"),
            "error must name the unknown account key: {err}"
        );
    }

    #[test]
    fn account_real_world_keys_parse() {
        // The keys a real Anchor IDL puts on an instruction-account entry
        // (verified against `brimigs-anchor-escrow`, the `quarry_*` programs,
        // and an AMM capstone program): name, docs, writable, signer,
        // optional, address, pda, relations. None of them may be rejected.
        let acc: IdlAccount = serde_json::from_str(
            r#"{
                "name": "user",
                "docs": ["the payer"],
                "writable": true,
                "signer": true,
                "optional": false,
                "relations": ["config"]
            }"#,
        )
        .unwrap();
        assert_eq!(acc.name, "user");
        assert_eq!(acc.docs, vec!["the payer".to_string()]);
        assert!(acc.writable);
        assert!(acc.signer);
        assert_eq!(acc.relations, vec!["config".to_string()]);
    }

    #[test]
    fn every_committed_idl_fixture_still_parses() {
        // Corpus regression for `deny_unknown_fields`: every real Anchor IDL
        // fixture committed in either crate must still parse after modeling
        // the account-level keys above. `truncated.json` is deliberately
        // invalid JSON (it exercises the bad-JSON error path elsewhere in
        // `idl_bundles.rs`), so it's excluded here.
        let dirs = [
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/idls"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../anchor-litesvm/idls"),
        ];
        let mut checked = 0;
        for dir in dirs {
            // A dir may not exist (the sibling crate ships its own fixtures);
            // the non-empty assertion below keeps this from passing vacuously.
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if path.file_name().and_then(|n| n.to_str()) == Some("truncated.json") {
                    continue;
                }
                let json = std::fs::read_to_string(&path).unwrap();
                Idl::parse(&json).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                checked += 1;
            }
        }
        assert!(
            checked >= 6,
            "expected to check several fixture IDLs, only found {checked}"
        );
    }
}
