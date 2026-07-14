//! Account role classification for `bundles_from_idl!`.
//!
//! Every account in an instruction's IDL entry plays one of three roles
//! from the bundle's point of view: a fixed program address the caller
//! never supplies, a PDA the emitter can derive from other accounts, or
//! a plain field the caller must populate. This module sorts IDL
//! accounts into those roles and orders the derivable PDAs so Task 8's
//! emitter can derive each one after its dependencies exist.

use super::idl::{IdlInstruction, IdlPda, IdlSeed};

/// The program a PDA derives under, when it is not the IDL's own program. An
/// `associated_token::` account carries a `Const` (the ATA program's 32 bytes);
/// a PDA whose deriving program is held in another account carries an `Account`
/// (that account's name, which resolves like any other seed dependency).
#[derive(Debug, Clone)]
pub enum DeriveProgram {
    Const(Vec<u8>),
    Account(String),
}

/// Why an account ended up a caller-supplied [`Role::Field`], recorded so the
/// emitter can explain each generated bundle field with a doc comment instead
/// of leaving an unexplained pubkey. Each variant corresponds to exactly one
/// demotion site in [`classify`]; see that function's doc for the fixpoint
/// this partitions.
#[derive(Debug, Clone)]
pub enum FieldReason {
    /// No `pda` at all: an ordinary account the caller always supplies.
    Plain,
    /// A seed reads an instruction argument (`path` is the arg name), which
    /// isn't known until the instruction is built, so the PDA can't be
    /// derived up front.
    ArgSeed { path: String },
    /// A seed reads another account's data (a dotted path like
    /// `vault_state.owner`), unresolvable from pubkeys alone.
    DataPathSeed { path: String },
    /// The deriving program itself (`pda.program`, not a seed) is an arg or a
    /// dotted data path, for the same reason as `ArgSeed`/`DataPathSeed`.
    ProgramDemoted,
    /// A seed names an account that isn't in this instruction at all.
    UnresolvedSeedTarget { name: String },
    /// This account's derivation and another's form a cycle (each waits on
    /// the other), so the fixpoint stalls with neither resolved.
    SeedCycle,
}

/// What one IDL account contributes to a bundle.
#[derive(Debug, Clone)]
pub enum Role {
    /// Fixed address in the IDL: injected, never a bundle field.
    Injected { address: String },
    /// PDA derivable from bundle fields (and previously derived PDAs).
    /// `deps` are account names this PDA's seeds (and, for an account-held
    /// deriving program, that program account) reference. `program` is the
    /// foreign program to derive under, or `None` for the IDL's own program.
    Derived {
        seeds: Vec<IdlSeed>,
        deps: Vec<String>,
        program: Option<DeriveProgram>,
    },
    /// Caller-supplied bundle field.
    Field { optional: bool, reason: FieldReason },
}

/// Classification for one instruction: every account's [`Role`] in IDL
/// order, plus the order `Derived` accounts must be built in so each
/// PDA's dependencies already exist.
#[derive(Debug)]
pub struct Classified {
    pub roles: Vec<(String, Role)>,
    pub derivation_order: Vec<String>,
}

/// Classify every account in `ix` as [`Role::Injected`], [`Role::Derived`],
/// or [`Role::Field`].
///
/// PDAs are resolved by fixpoint: a PDA becomes `Derived` once every
/// account its seeds reference has already been classified (as `Field`,
/// `Injected`, or an earlier `Derived`), which is what lets chains like
/// `vault` (seeded on `vault_state`) which is itself seeded on `user`
/// resolve in dependency order. A PDA seeded on an instruction arg or a
/// dotted account-data path (e.g. `vault_state.owner`) can't be derived
/// from pubkeys alone and is demoted to `Field` up front. Anything still
/// unresolved when the fixpoint stalls — a cycle between two PDAs, or a
/// seed path naming an account outside this instruction — also demotes
/// to `Field`: the caller supplies the key rather than the build
/// guessing at a shape it doesn't understand.
///
/// # Errors
///
/// Returns a message (which the macro surfaces as a `syn::Error`) when an
/// account's `pda.program` is a const that isn't a 32-byte program id. That
/// shape can only be a malformed IDL, so it is refused loudly rather than
/// degraded: demoting it would silently swallow a detected corruption. The
/// message names the account and the actual byte length. (An arg or dotted
/// program path is a legitimate shape the macro can't resolve, so it demotes
/// to a field like an unresolvable seed, not an error.)
pub fn classify(ix: &IdlInstruction) -> Result<Classified, String> {
    let mut roles: Vec<Option<Role>> = vec![None; ix.accounts.len()];
    let mut pending: Vec<usize> = Vec::new();

    for (i, acc) in ix.accounts.iter().enumerate() {
        if let Some(address) = &acc.address {
            roles[i] = Some(Role::Injected {
                address: address.clone(),
            });
            continue;
        }
        let Some(pda) = &acc.pda else {
            roles[i] = Some(Role::Field {
                optional: acc.optional,
                reason: FieldReason::Plain,
            });
            continue;
        };
        let unresolvable_seed_reason = pda.seeds.iter().find_map(|s| match s {
            IdlSeed::Arg { path } => Some(FieldReason::ArgSeed {
                path: path.clone().unwrap_or_default(),
            }),
            IdlSeed::Account { path, .. } if path.contains('.') => {
                Some(FieldReason::DataPathSeed { path: path.clone() })
            }
            _ => None,
        });
        let program_class = classify_program(&pda.program);
        if let ProgramClass::Malformed { len } = program_class {
            return Err(format!(
                "bundles_from_idl!: account `{}` has a pda.program const of {len} bytes, but a \
                 program id is 32 bytes, so this IDL is malformed",
                acc.name
            ));
        }
        let reason = unresolvable_seed_reason.or(match program_class {
            ProgramClass::Demote => Some(FieldReason::ProgramDemoted),
            _ => None,
        });
        if let Some(reason) = reason {
            roles[i] = Some(Role::Field {
                optional: acc.optional,
                reason,
            });
        } else {
            pending.push(i);
        }
    }

    let mut derivation_order = Vec::new();
    loop {
        let mut progressed = false;
        let mut still_pending = Vec::new();
        for i in pending {
            let pda = ix.accounts[i]
                .pda
                .as_ref()
                .expect("pending accounts are PDAs by construction");
            let deps = pda_dep_names(pda);
            let program = match classify_program(&pda.program) {
                ProgramClass::Own => None,
                ProgramClass::Foreign(DeriveProgram::Account(path)) => {
                    Some(DeriveProgram::Account(path))
                }
                ProgramClass::Foreign(program @ DeriveProgram::Const(_)) => Some(program),
                ProgramClass::Demote | ProgramClass::Malformed { .. } => {
                    unreachable!("demoted programs are Field, malformed ones already errored")
                }
            };
            let all_resolved = deps.iter().all(|dep| {
                ix.accounts
                    .iter()
                    .position(|a| &a.name == dep)
                    .is_some_and(|idx| roles[idx].is_some())
            });
            if all_resolved {
                let seeds = pda.seeds.clone();
                derivation_order.push(ix.accounts[i].name.clone());
                roles[i] = Some(Role::Derived {
                    seeds,
                    deps,
                    program,
                });
                progressed = true;
            } else {
                still_pending.push(i);
            }
        }
        pending = still_pending;
        if !progressed {
            break;
        }
    }

    // Whatever's left is a cycle or a seed naming an unknown account: both
    // are shapes the build can't resolve, so the caller supplies the key.
    // `stall_reason` tells the two apart by replaying the same dependency
    // names the fixpoint above used.
    for i in pending {
        let pda = ix.accounts[i]
            .pda
            .as_ref()
            .expect("pending accounts are PDAs by construction");
        roles[i] = Some(Role::Field {
            optional: ix.accounts[i].optional,
            reason: stall_reason(ix, pda),
        });
    }

    let roles = ix
        .accounts
        .iter()
        .zip(roles)
        .map(|(acc, role)| (acc.name.clone(), role.expect("every account classified")))
        .collect();

    Ok(Classified {
        roles,
        derivation_order,
    })
}

/// How a PDA's deriving program resolves at build time.
enum ProgramClass {
    /// No `program` key: derive under the IDL's own program.
    Own,
    /// A `program` the emitter can name: const bytes or an account it derives.
    Foreign(DeriveProgram),
    /// A legitimate `program` shape the macro can't turn into a pubkey at build
    /// time (an arg, or a dotted account-data path): the account demotes to a
    /// caller-supplied field, the same rule an unresolvable seed follows.
    Demote,
    /// A const `program` that isn't a 32-byte program id (`len` is what it is).
    /// No legitimate Anchor output produces this, so it's a detected corruption
    /// the macro refuses rather than silently degrades.
    Malformed { len: usize },
}

fn classify_program(program: &Option<IdlSeed>) -> ProgramClass {
    match program {
        None => ProgramClass::Own,
        Some(IdlSeed::Const { value }) if value.len() == 32 => {
            ProgramClass::Foreign(DeriveProgram::Const(value.clone()))
        }
        Some(IdlSeed::Const { value }) => ProgramClass::Malformed { len: value.len() },
        Some(IdlSeed::Account { path, .. }) if !path.contains('.') => {
            ProgramClass::Foreign(DeriveProgram::Account(path.clone()))
        }
        Some(IdlSeed::Account { .. }) | Some(IdlSeed::Arg { .. }) => ProgramClass::Demote,
    }
}

/// The account names one PDA's derivation depends on: every `Account`-kind
/// seed, plus (if the deriving program itself lives in another account) that
/// account too. Shared by the fixpoint loop, which uses it to check whether a
/// pending PDA's dependencies are all resolved, and by [`stall_reason`],
/// which replays it to explain a PDA the fixpoint never resolved.
fn pda_dep_names(pda: &IdlPda) -> Vec<String> {
    let mut deps: Vec<String> = pda
        .seeds
        .iter()
        .filter_map(|s| match s {
            IdlSeed::Account { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect();
    if let ProgramClass::Foreign(DeriveProgram::Account(path)) = classify_program(&pda.program) {
        if !deps.contains(&path) {
            deps.push(path);
        }
    }
    deps
}

/// Explain a PDA still `pending` after the fixpoint stalled: a dependency
/// name that isn't any account in this instruction is
/// [`FieldReason::UnresolvedSeedTarget`]; otherwise every dependency exists
/// but never resolved, meaning it's part of the same stalled batch, a
/// [`FieldReason::SeedCycle`].
fn stall_reason(ix: &IdlInstruction, pda: &IdlPda) -> FieldReason {
    match pda_dep_names(pda)
        .into_iter()
        .find(|dep| !ix.accounts.iter().any(|a| &a.name == dep))
    {
        Some(name) => FieldReason::UnresolvedSeedTarget { name },
        None => FieldReason::SeedCycle,
    }
}

#[cfg(test)]
mod tests {
    use super::super::idl::{Idl, IdlAccount, IdlInstruction, IdlPda, IdlSeed};
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
    fn vault_close_classifies() {
        let idl = vault();
        let close = idl.instructions.iter().find(|i| i.name == "close").unwrap();
        let c = classify(close).unwrap();
        let role = |n: &str| &c.roles.iter().find(|(name, _)| name == n).unwrap().1;
        assert!(matches!(
            role("user"),
            Role::Field {
                optional: false,
                reason: FieldReason::Plain
            }
        ));
        assert!(matches!(role("system_program"), Role::Injected { .. }));
        // vault_state depends on user; vault depends on vault_state: both derive.
        assert!(matches!(role("vault_state"), Role::Derived { .. }));
        assert!(matches!(role("vault"), Role::Derived { .. }));
        // Chain order: vault_state before vault.
        assert_eq!(c.derivation_order, vec!["vault_state", "vault"]);
    }

    /// A plain caller-supplied account (no address, no pda).
    fn field(name: &str) -> IdlAccount {
        IdlAccount {
            name: name.into(),
            optional: false,
            address: None,
            pda: None,
            ..Default::default()
        }
    }

    fn account_seed(path: &str) -> IdlSeed {
        IdlSeed::Account {
            path: path.into(),
            account: None,
        }
    }

    #[test]
    fn arg_seed_demotes_to_field() {
        // Hand-built ix: a PDA seeded on an instruction arg is not derivable
        // from pubkeys alone; it must stay a bundle field.
        let ix = IdlInstruction {
            name: "make".into(),
            accounts: vec![IdlAccount {
                name: "escrow".into(),
                optional: false,
                address: None,
                pda: Some(IdlPda {
                    seeds: vec![IdlSeed::Arg { path: None }],
                    program: None,
                }),
                ..Default::default()
            }],
        };
        let c = classify(&ix).unwrap();
        assert!(matches!(
            c.roles[0].1,
            Role::Field {
                reason: FieldReason::ArgSeed { .. },
                ..
            }
        ));
    }

    #[test]
    fn dotted_account_path_demotes_to_field() {
        // `path: "vault_state.owner"` reads account DATA, unresolvable at
        // build time; demote to Field.
        let ix = IdlInstruction {
            name: "x".into(),
            accounts: vec![IdlAccount {
                name: "p".into(),
                optional: false,
                address: None,
                pda: Some(IdlPda {
                    seeds: vec![account_seed("vault_state.owner")],
                    program: None,
                }),
                ..Default::default()
            }],
        };
        let c = classify(&ix).unwrap();
        assert!(matches!(
            &c.roles[0].1,
            Role::Field {
                reason: FieldReason::DataPathSeed { path },
                ..
            } if path == "vault_state.owner"
        ));
    }

    #[test]
    fn const_program_pda_records_the_program_bytes() {
        // An ATA-style account: seeded on Fields, deriving under a const program.
        // It stays Derived and carries the program's bytes so the emitter can
        // derive under them instead of the host program.
        let ix = IdlInstruction {
            name: "deposit".into(),
            accounts: vec![
                field("user"),
                field("mint_x"),
                IdlAccount {
                    name: "user_x".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![account_seed("user"), account_seed("mint_x")],
                        program: Some(IdlSeed::Const {
                            value: vec![7u8; 32],
                        }),
                    }),
                    ..Default::default()
                },
            ],
        };
        let c = classify(&ix).unwrap();
        let user_x = &c.roles.iter().find(|(n, _)| n == "user_x").unwrap().1;
        assert!(
            matches!(user_x, Role::Derived { program: Some(DeriveProgram::Const(b)), .. } if b == &vec![7u8; 32])
        );
    }

    #[test]
    fn account_program_becomes_a_dependency() {
        // A PDA whose deriving program is held in another account: that account
        // resolves like a seed dep and must appear in `deps` so the emitter binds
        // it as the derive program.
        let ix = IdlInstruction {
            name: "x".into(),
            accounts: vec![
                field("user"),
                field("token_program"),
                IdlAccount {
                    name: "vault".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![account_seed("user")],
                        program: Some(account_seed("token_program")),
                    }),
                    ..Default::default()
                },
            ],
        };
        let c = classify(&ix).unwrap();
        let vault = &c.roles.iter().find(|(n, _)| n == "vault").unwrap().1;
        let Role::Derived { deps, program, .. } = vault else {
            panic!("vault must derive");
        };
        assert!(deps.iter().any(|d| d == "token_program"));
        assert!(matches!(program, Some(DeriveProgram::Account(p)) if p == "token_program"));
    }

    #[test]
    fn dotted_program_path_demotes_to_field() {
        // A deriving program read from account DATA can't become a pubkey at
        // build time; demote the account, same as an unresolvable seed.
        let ix = IdlInstruction {
            name: "x".into(),
            accounts: vec![
                field("user"),
                IdlAccount {
                    name: "vault".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![account_seed("user")],
                        program: Some(account_seed("cfg.program")),
                    }),
                    ..Default::default()
                },
            ],
        };
        let c = classify(&ix).unwrap();
        assert!(matches!(
            c.roles[1].1,
            Role::Field {
                reason: FieldReason::ProgramDemoted,
                ..
            }
        ));
    }

    #[test]
    fn unresolved_seed_target_demotes_to_field() {
        // A seed names an account that isn't in this instruction at all: not
        // a cycle, just an IDL shape this build can't resolve.
        let ix = IdlInstruction {
            name: "x".into(),
            accounts: vec![IdlAccount {
                name: "vault".into(),
                optional: false,
                address: None,
                pda: Some(IdlPda {
                    seeds: vec![account_seed("ghost")],
                    program: None,
                }),
                ..Default::default()
            }],
        };
        let c = classify(&ix).unwrap();
        assert!(matches!(
            &c.roles[0].1,
            Role::Field {
                reason: FieldReason::UnresolvedSeedTarget { name },
                ..
            } if name == "ghost"
        ));
    }

    #[test]
    fn seed_cycle_demotes_to_field() {
        // `a` is seeded on `b` and `b` is seeded on `a`: both accounts exist,
        // but the fixpoint stalls forever, so both demote to Field.
        let ix = IdlInstruction {
            name: "x".into(),
            accounts: vec![
                IdlAccount {
                    name: "a".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![account_seed("b")],
                        program: None,
                    }),
                    ..Default::default()
                },
                IdlAccount {
                    name: "b".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![account_seed("a")],
                        program: None,
                    }),
                    ..Default::default()
                },
            ],
        };
        let c = classify(&ix).unwrap();
        assert!(matches!(
            &c.roles[0].1,
            Role::Field {
                reason: FieldReason::SeedCycle,
                ..
            }
        ));
        assert!(matches!(
            &c.roles[1].1,
            Role::Field {
                reason: FieldReason::SeedCycle,
                ..
            }
        ));
    }

    #[test]
    fn non_32_byte_program_const_errors() {
        // A const `program` that isn't 32 bytes can only be a malformed IDL. The
        // macro refuses it (naming the account and the actual length) rather than
        // demoting it to a field and swallowing the corruption.
        let ix = IdlInstruction {
            name: "x".into(),
            accounts: vec![
                field("user"),
                IdlAccount {
                    name: "user_x".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![account_seed("user")],
                        program: Some(IdlSeed::Const {
                            value: vec![1u8; 31],
                        }),
                    }),
                    ..Default::default()
                },
            ],
        };
        let err = classify(&ix).unwrap_err();
        assert!(
            err.contains("user_x") && err.contains("31 bytes"),
            "error must name the account and the actual byte length: {err}"
        );
    }

    // The associated-token and SPL-token program ids, raw. An ATA's canonical
    // address is `find_program_address([wallet, token_program, mint], ATA)`.
    const ATA_PROGRAM: [u8; 32] = [
        140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153,
        218, 255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
    ];
    const TOKEN_PROGRAM: [u8; 32] = [
        6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133,
        237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
    ];

    /// Replay a `Role::Derived` the way the emitter would: substitute each
    /// account seed with the concrete pubkey `resolve` gives it, derive under the
    /// captured program (or `host` when it carries none). This exercises what
    /// classify captured, so an address mismatch means the wrong program or seeds
    /// were recorded.
    fn replay(
        role: &Role,
        resolve: &impl Fn(&str) -> solana_program::pubkey::Pubkey,
        host: solana_program::pubkey::Pubkey,
    ) -> solana_program::pubkey::Pubkey {
        use solana_program::pubkey::Pubkey;
        let Role::Derived { seeds, program, .. } = role else {
            panic!("not a Derived role");
        };
        let bytes: Vec<Vec<u8>> = seeds
            .iter()
            .map(|s| match s {
                IdlSeed::Const { value } => value.clone(),
                IdlSeed::Account { path, .. } => resolve(path).to_bytes().to_vec(),
                IdlSeed::Arg { .. } => unreachable!("Derived accounts have no arg seeds"),
            })
            .collect();
        let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
        let prog = match program {
            None => host,
            Some(DeriveProgram::Const(b)) => Pubkey::new_from_array((&b[..]).try_into().unwrap()),
            Some(DeriveProgram::Account(p)) => resolve(p),
        };
        Pubkey::find_program_address(&refs, &prog).0
    }

    #[test]
    fn const_program_derivation_matches_the_ata_address() {
        use solana_program::pubkey::Pubkey;
        let user = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let host = Pubkey::new_unique();

        // The ATA shape the AMM probe surfaced: [wallet, token_program_bytes, mint]
        // deriving under the ATA program.
        let ix = IdlInstruction {
            name: "deposit".into(),
            accounts: vec![
                field("user"),
                field("mint_x"),
                IdlAccount {
                    name: "user_x".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![
                            account_seed("user"),
                            IdlSeed::Const {
                                value: TOKEN_PROGRAM.to_vec(),
                            },
                            account_seed("mint_x"),
                        ],
                        program: Some(IdlSeed::Const {
                            value: ATA_PROGRAM.to_vec(),
                        }),
                    }),
                    ..Default::default()
                },
            ],
        };
        let c = classify(&ix).unwrap();
        let user_x = &c.roles.iter().find(|(n, _)| n == "user_x").unwrap().1;

        let resolve = |name: &str| match name {
            "user" => user,
            "mint_x" => mint,
            other => panic!("unexpected seed account {other}"),
        };
        let actual = replay(user_x, &resolve, host);

        // The independently-built canonical ATA address.
        let (expected, _) = Pubkey::find_program_address(
            &[user.as_ref(), TOKEN_PROGRAM.as_ref(), mint.as_ref()],
            &Pubkey::new_from_array(ATA_PROGRAM),
        );
        assert_eq!(actual, expected, "must derive under the ATA program");

        // The bug this closes: deriving under the host program yields a different,
        // wrong address, which is exactly what the old code produced.
        let (under_host, _) = Pubkey::find_program_address(
            &[user.as_ref(), TOKEN_PROGRAM.as_ref(), mint.as_ref()],
            &host,
        );
        assert_ne!(actual, under_host, "must NOT derive under the host program");
    }

    #[test]
    fn host_pda_seeded_on_foreign_ata_orders_after_it() {
        // Field -> foreign-Derived -> host-Derived: `user_ata` derives under the
        // ATA program from Fields; `receipt` derives under the host program from
        // `user_ata`. The chain must resolve with the foreign ATA built first.
        let ix = IdlInstruction {
            name: "claim".into(),
            accounts: vec![
                field("user"),
                field("mint"),
                IdlAccount {
                    name: "user_ata".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![account_seed("user"), account_seed("mint")],
                        program: Some(IdlSeed::Const {
                            value: vec![9u8; 32],
                        }),
                    }),
                    ..Default::default()
                },
                IdlAccount {
                    name: "receipt".into(),
                    optional: false,
                    address: None,
                    pda: Some(IdlPda {
                        seeds: vec![
                            IdlSeed::Const {
                                value: b"receipt".to_vec(),
                            },
                            account_seed("user_ata"),
                        ],
                        program: None,
                    }),
                    ..Default::default()
                },
            ],
        };
        let c = classify(&ix).unwrap();
        assert_eq!(c.derivation_order, vec!["user_ata", "receipt"]);
        let receipt = &c.roles.iter().find(|(n, _)| n == "receipt").unwrap().1;
        assert!(matches!(receipt, Role::Derived { program: None, .. }));
    }
}
