//! The `bundles_from_idl!` emitter.
//!
//! Given a committed Anchor IDL and the `client::{accounts, args}` types
//! `declare_program!` generates from that same file, this emits, per
//! instruction, a caller-facing pubkey bundle plus the wiring that turns it
//! into a real instruction:
//!
//! - `pub struct <Ix>Bundle` with one `Pubkey` field per account the caller
//!   must supply (accounts the IDL fixes to an address, or derives as a PDA
//!   from other accounts, are not fields).
//! - `impl Default for <Ix>Bundle`, filling each field with
//!   `Pubkey::new_unique()` so a test binds only what it cares about, with a
//!   well-known-name policy for `token_program` / `associated_token_program`.
//! - `impl From<<Ix>Bundle> for <prog>::client::accounts::<Ix>`, which binds
//!   the caller's fields, derives every PDA in dependency order, and injects
//!   the fixed addresses.
//! - `impl BuildableIx<<Ix>Bundle> for <prog>::client::args::<Ix>`, the
//!   type-level pairing `anchor_litesvm`'s `Program::build_ix` consumes.
//! - `pub fn <account>_pda(<root fields>: &Pubkey, ...) -> (Pubkey, u8)` per
//!   derivable PDA, recomputing its whole seed chain from the root fields.
//! - one module-level `pub fn injected_programs() -> Vec<(Pubkey, &'static
//!   str)>`, the fixed addresses unioned across every instruction.
//!
//! Every emitted pubkey type is `::anchor_lang::prelude::Pubkey`, matching the
//! type the generated `client::accounts` fields carry, so the `From` impl type
//! checks against them.

use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

use crate::classify::{classify, Classified, DeriveProgram, FieldReason, Role};
use crate::idl::{Idl, IdlInstruction, IdlSeed};

/// `bundles_from_idl!(<name>)` or `bundles_from_idl!(<name>, "path/to.json")`.
struct Args {
    name: Ident,
    path: Option<String>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let path = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let lit: LitStr = input.parse()?;
            Some(lit.value())
        } else {
            None
        };
        Ok(Args { name, path })
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let args: Args = syn::parse2(input)?;
    let name_span = args.name.span();

    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|e| {
        syn::Error::new(
            name_span,
            format!("bundles_from_idl!: CARGO_MANIFEST_DIR unset: {e}"),
        )
    })?;
    let rel = args
        .path
        .clone()
        .unwrap_or_else(|| format!("idls/{}.json", args.name));
    let full = std::path::Path::new(&manifest).join(&rel);
    let json = std::fs::read_to_string(&full).map_err(|e| {
        syn::Error::new(
            name_span,
            format!("bundles_from_idl!: cannot read {}: {e}", full.display()),
        )
    })?;
    let idl = Idl::parse(&json).map_err(|e| {
        syn::Error::new(
            name_span,
            format!(
                "bundles_from_idl!: invalid IDL JSON in {}: {e}",
                full.display()
            ),
        )
    })?;

    let prog = &args.name;
    let address = &idl.address;
    let program_id_const = quote! {
        const PROGRAM_ID: ::anchor_lang::prelude::Pubkey =
            ::anchor_lang::prelude::Pubkey::from_str_const(#address);
    };

    let mut items = Vec::with_capacity(idl.instructions.len());
    // PDA helpers are named by account, and one account (a `vault` PDA) recurs
    // across instructions; collect them keyed by helper name so a name that
    // derives identically everywhere is emitted once, and a name that derives
    // two different ways surfaces as an error rather than a duplicate `fn`.
    let mut helpers: Vec<Helper> = Vec::new();
    for ix in &idl.instructions {
        let c = classify(ix).map_err(|msg| syn::Error::new(name_span, msg))?;
        items.push(emit_instruction(prog, ix, &c));
        for name in &c.derivation_order {
            record_helper(&mut helpers, name, emit_pda_helper(&c, name))?;
        }
    }
    let helper_tokens = helpers.iter().map(|h| &h.tokens);
    let injected = emit_injected_programs(&idl);

    Ok(quote! {
        #program_id_const
        #(#items)*
        #(#helper_tokens)*
        #injected
    })
}

/// One deduplicated PDA helper: its `fn <account>_pda` name, the token stream,
/// and that stream's textual form (so a re-derivation of the same account can
/// be checked for an identical body before it is dropped as a duplicate).
struct Helper {
    name: String,
    text: String,
    tokens: TokenStream,
}

/// Fold one instruction's `<account>_pda` helper into the module-wide set. A
/// second sighting with a byte-identical body is a harmless duplicate (the same
/// PDA seen in another instruction) and is dropped; a second sighting with a
/// different body means one account name derives two ways across the IDL, which
/// can't collapse to a single free function, so it errors.
fn record_helper(helpers: &mut Vec<Helper>, account: &str, tokens: TokenStream) -> syn::Result<()> {
    let name = format!("{account}_pda");
    let text = tokens.to_string();
    if let Some(existing) = helpers.iter().find(|h| h.name == name) {
        if existing.text != text {
            return Err(syn::Error::new(
                Span::call_site(),
                format!(
                    "bundles_from_idl!: account `{account}` derives differently across \
                     instructions, so `{name}` can't be a single helper; supply this account \
                     as a bundle field instead"
                ),
            ));
        }
        return Ok(());
    }
    helpers.push(Helper { name, text, tokens });
    Ok(())
}

/// Everything one instruction contributes: the bundle type, its `Default` and
/// `From`, and the `BuildableIx` pairing. PDA helpers are hoisted to module
/// scope by the caller (they dedup across instructions).
fn emit_instruction(prog: &Ident, ix: &IdlInstruction, c: &Classified) -> TokenStream {
    let ix_pascal = pascal_case(&ix.name);
    let ix_ty = ident(&ix_pascal);
    let bundle_ident = ident(&format!("{ix_pascal}Bundle"));

    let (field_defs, default_inits) = bundle_fields(c);
    let from_impl = emit_from_impl(prog, c, &bundle_ident, &ix_ty);

    quote! {
        #[derive(Copy, Clone, Debug)]
        pub struct #bundle_ident {
            #(#field_defs),*
        }

        impl ::core::default::Default for #bundle_ident {
            fn default() -> Self {
                Self { #(#default_inits),* }
            }
        }

        #from_impl

        impl ::anchor_litesvm::BuildableIx<#bundle_ident> for #prog::client::args::#ix_ty {
            type Accounts = #prog::client::accounts::#ix_ty;
        }

        // Generated bundles derive their PDAs eagerly in `From`, so the
        // pre-projection hook `Tx::build` runs has nothing left to do.
        impl ::anchor_litesvm::Resolvable for #bundle_ident {
            fn resolve_all(&mut self, _ctx: &::anchor_litesvm::AnchorContext) {}
        }
    }
}

/// The bundle's field declarations and their `Default` initializers, one per
/// `Role::Field`. Optional accounts become `Option<Pubkey>` (default `None`);
/// required ones default to a fresh placeholder, except the well-known
/// program-name policy. Each field carries a `#[doc]` explaining what it is:
/// an ordinary caller-supplied account, or (for a PDA the emitter couldn't
/// derive) which of `classify`'s demotion reasons put it here, so a bundle
/// never shows an unexplained pubkey.
fn bundle_fields(c: &Classified) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut field_defs = Vec::new();
    let mut default_inits = Vec::new();
    for (name, role) in &c.roles {
        let Role::Field { optional, reason } = role else {
            continue;
        };
        let f = ident(name);
        let doc_attrs = field_docs(name, reason)
            .into_iter()
            .map(|d| quote! { #[doc = #d] });
        if *optional {
            field_defs.push(quote! {
                #(#doc_attrs)*
                pub #f: ::core::option::Option<::anchor_lang::prelude::Pubkey>
            });
            default_inits.push(quote! { #f: ::core::option::Option::None });
        } else {
            field_defs.push(quote! {
                #(#doc_attrs)*
                pub #f: ::anchor_lang::prelude::Pubkey
            });
            let default = well_known_default(name);
            default_inits.push(quote! { #f: #default });
        }
    }
    (field_defs, default_inits)
}

/// The `#[doc]` lines for one bundle field: a sentence naming why the
/// emitter couldn't derive it (or, for a plain account, that it's just
/// caller-supplied), plus a well-known-default note for `token_program` /
/// `associated_token_program` when it applies.
fn field_docs(name: &str, reason: &FieldReason) -> Vec<String> {
    let mut docs = vec![reason_doc(reason)];
    if let Some(note) = well_known_note(name) {
        docs.push(note.to_string());
    }
    docs
}

/// One sentence naming why `classify` demoted this account to a bundle
/// field, mirroring [`FieldReason`] one variant at a time.
fn reason_doc(reason: &FieldReason) -> String {
    match reason {
        FieldReason::Plain => "Caller-supplied.".to_string(),
        FieldReason::ArgSeed { path } => format!(
            "Caller-supplied: this PDA's seeds reference instruction arg `{path}`, so it \
             cannot be derived at build time; compute it with `find_program_address` or reuse \
             your fixture's address."
        ),
        FieldReason::DataPathSeed { path } => format!(
            "Caller-supplied: this PDA's seeds reference account data (`{path}`), which isn't \
             known until that account exists on chain, so it cannot be derived at build time; \
             compute it with `find_program_address` or reuse your fixture's address."
        ),
        FieldReason::ProgramDemoted => "Caller-supplied: this PDA's deriving program is an \
             instruction arg or account-data path, so it cannot be resolved at build time; \
             compute it with `find_program_address` or reuse your fixture's address."
            .to_string(),
        FieldReason::UnresolvedSeedTarget { name } => format!(
            "Caller-supplied: this PDA's seeds reference `{name}`, which isn't an account in \
             this instruction, so it cannot be derived here."
        ),
        FieldReason::SeedCycle => "Caller-supplied: this PDA's derivation forms a cycle with \
             another account's, so neither can be derived here."
            .to_string(),
    }
}

/// The well-known-default note for a bundle field whose name matches one of
/// the [`well_known_default`] program-name policies, or `None` for any other
/// field name.
fn well_known_note(name: &str) -> Option<&'static str> {
    match name {
        "token_program" => {
            Some("Defaults to the classic SPL Token program; override for Token-2022.")
        }
        "associated_token_program" => {
            Some("Defaults to the Associated Token Account program; override to derive under a different implementation.")
        }
        _ => None,
    }
}

/// `From<Bundle>`: bind the caller's fields and the fixed addresses as locals,
/// derive each PDA in dependency order, then move every account into the
/// generated accounts struct by field-init shorthand.
fn emit_from_impl(
    prog: &Ident,
    c: &Classified,
    bundle_ident: &Ident,
    ix_ty: &Ident,
) -> TokenStream {
    let mut lets = Vec::new();
    for (name, role) in &c.roles {
        let f = ident(name);
        match role {
            Role::Field { .. } => lets.push(quote! { let #f = __bundle.#f; }),
            Role::Injected { address } => lets.push(quote! {
                let #f = ::anchor_lang::prelude::Pubkey::from_str_const(#address);
            }),
            // Derived accounts are computed below, in dependency order.
            Role::Derived { .. } => {}
        }
    }
    for name in &c.derivation_order {
        let f = ident(name);
        let Role::Derived { seeds, program, .. } = role_of(c, name) else {
            unreachable!("derivation_order only names Derived accounts");
        };
        let seed_exprs = seeds.iter().map(seed_expr);
        // Every account here is a local `Pubkey` value, so an account-held
        // program is referenced (`roots` empty).
        let prog = program_expr(program, &[]);
        lets.push(quote! {
            let #f = ::anchor_lang::prelude::Pubkey::find_program_address(
                &[#(#seed_exprs),*],
                #prog,
            ).0;
        });
    }
    let field_names = c.roles.iter().map(|(name, _)| ident(name));

    quote! {
        impl ::core::convert::From<#bundle_ident> for #prog::client::accounts::#ix_ty {
            fn from(__bundle: #bundle_ident) -> Self {
                #(#lets)*
                Self { #(#field_names),* }
            }
        }
    }
}

/// A standalone `<account>_pda` for one derivable PDA: parameters are the root
/// `Field` accounts its seed chain bottoms out in, and the body recomputes
/// every intermediate PDA before returning the target's full
/// `(Pubkey, u8)` tuple.
fn emit_pda_helper(c: &Classified, target: &str) -> TokenStream {
    let Role::Derived { deps, .. } = role_of(c, target) else {
        unreachable!("emit_pda_helper is only called for Derived accounts");
    };
    // Walk from the target's dependencies (not the target itself), so the
    // target is never recorded as an intermediate to recompute.
    let mut roots = Vec::new();
    let mut injected = Vec::new();
    let mut intermediates = Vec::new();
    for dep in deps {
        collect_cone(c, dep, &mut roots, &mut injected, &mut intermediates);
    }

    let params = roots.iter().map(|name| {
        let p = ident(name);
        quote! { #p: &::anchor_lang::prelude::Pubkey }
    });
    let injected_lets = injected.iter().map(|name| {
        let f = ident(name);
        let Role::Injected { address } = role_of(c, name) else {
            unreachable!("collect_cone only routes Injected accounts here");
        };
        quote! {
            let #f = ::anchor_lang::prelude::Pubkey::from_str_const(#address);
        }
    });
    let intermediate_lets = intermediates.iter().map(|name| {
        let f = ident(name);
        let Role::Derived { seeds, program, .. } = role_of(c, name) else {
            unreachable!("intermediates are Derived by construction");
        };
        let seed_exprs = seeds.iter().map(seed_expr);
        let prog = program_expr(program, &roots);
        quote! {
            let #f = ::anchor_lang::prelude::Pubkey::find_program_address(
                &[#(#seed_exprs),*],
                #prog,
            ).0;
        }
    });

    let helper = ident(&format!("{target}_pda"));
    let Role::Derived { seeds, program, .. } = role_of(c, target) else {
        unreachable!("emit_pda_helper is only called for Derived accounts");
    };
    let seed_exprs = seeds.iter().map(seed_expr);
    let prog = program_expr(program, &roots);

    quote! {
        pub fn #helper(#(#params),*) -> (::anchor_lang::prelude::Pubkey, u8) {
            #(#injected_lets)*
            #(#intermediate_lets)*
            ::anchor_lang::prelude::Pubkey::find_program_address(
                &[#(#seed_exprs),*],
                #prog,
            )
        }
    }
}

/// Walk one dependency of a PDA's seed chain, sorting what it reaches into the
/// helper's parameters and body. A `Field` is a root parameter; an `Injected`
/// account is a fixed address to bind locally; a `Derived` account is an
/// intermediate PDA recorded post-order (after its own dependencies) so the
/// emitted `let` bindings stay in computable order.
fn collect_cone(
    c: &Classified,
    name: &str,
    roots: &mut Vec<String>,
    injected: &mut Vec<String>,
    intermediates: &mut Vec<String>,
) {
    match role_of(c, name) {
        Role::Field { .. } => push_unique(roots, name),
        Role::Injected { .. } => push_unique(injected, name),
        Role::Derived { deps, .. } => {
            for dep in deps {
                collect_cone(c, dep, roots, injected, intermediates);
            }
            push_unique(intermediates, name);
        }
    }
}

fn push_unique(v: &mut Vec<String>, name: &str) {
    if !v.iter().any(|n| n == name) {
        v.push(name.to_string());
    }
}

/// A single seed as a Rust expression usable inside `find_program_address`'s
/// `&[..]`: a `Const` becomes the byte-string literal the program hashes, an
/// `Account` becomes `<binding>.as_ref()` (the binding is an in-scope `Pubkey`
/// or `&Pubkey`, both of which `.as_ref()` to `&[u8]`).
fn seed_expr(seed: &IdlSeed) -> TokenStream {
    match seed {
        IdlSeed::Const { value } => {
            let lit = Literal::byte_string(value);
            quote! { #lit }
        }
        IdlSeed::Account { path, .. } => {
            let binding = ident(path);
            quote! { #binding.as_ref() }
        }
        IdlSeed::Arg { .. } => {
            // classify demotes any account with an Arg seed to Field, so no
            // Derived account (the only kind whose seeds reach here) carries one.
            unreachable!("Derived accounts never have Arg seeds");
        }
    }
}

/// The deriving-program argument to a `find_program_address` call. `None` derives
/// under the IDL's own program; a `Const` derives under its 32 bytes verbatim (a
/// `Pubkey::new_from_array`, never re-encoded through base58); an `Account`
/// derives under an in-scope binding. That binding is a `Pubkey` value
/// everywhere except a helper's root parameters, which are already `&Pubkey`, so
/// `roots` names those to avoid a double reference.
fn program_expr(program: &Option<DeriveProgram>, roots: &[String]) -> TokenStream {
    match program {
        None => quote! { &PROGRAM_ID },
        Some(DeriveProgram::Const(bytes)) => {
            let elems = bytes.iter().map(|b| Literal::u8_suffixed(*b));
            quote! {
                &::anchor_lang::prelude::Pubkey::new_from_array([#(#elems),*])
            }
        }
        Some(DeriveProgram::Account(path)) => {
            let binding = ident(path);
            if roots.iter().any(|r| r == path) {
                quote! { #binding }
            } else {
                quote! { &#binding }
            }
        }
    }
}

/// The fixed addresses across every instruction, deduped by address, as
/// `injected_programs() -> Vec<(Pubkey, &'static str)>`. The `&str` is the IDL
/// account name (`"system_program"`), the label a caller matches on.
fn emit_injected_programs(idl: &Idl) -> TokenStream {
    let mut seen: Vec<String> = Vec::new();
    let mut entries = Vec::new();
    for ix in &idl.instructions {
        for acc in &ix.accounts {
            let Some(address) = &acc.address else {
                continue;
            };
            if seen.contains(address) {
                continue;
            }
            seen.push(address.clone());
            let name = &acc.name;
            entries.push(quote! {
                (::anchor_lang::prelude::Pubkey::from_str_const(#address), #name)
            });
        }
    }
    quote! {
        pub fn injected_programs()
            -> ::std::vec::Vec<(::anchor_lang::prelude::Pubkey, &'static str)>
        {
            ::std::vec![#(#entries),*]
        }
    }
}

/// The default value for a required bundle field: a fresh placeholder, unless
/// the account's name marks it as a well-known program whose real address is a
/// better default (still overridable via struct-update syntax).
fn well_known_default(name: &str) -> TokenStream {
    match name {
        "token_program" => quote! {
            ::anchor_lang::prelude::Pubkey::from_str_const(
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            )
        },
        "associated_token_program" => quote! {
            ::anchor_lang::prelude::Pubkey::from_str_const(
                "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
            )
        },
        _ => quote! { ::anchor_lang::prelude::Pubkey::new_unique() },
    }
}

fn role_of<'a>(c: &'a Classified, name: &str) -> &'a Role {
    &c.roles
        .iter()
        .find(|(n, _)| n == name)
        .expect("classify records every account")
        .1
}

fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

// These two error branches need a fixture file that actually exists on disk,
// which only holds under a plain `cargo test` (CARGO_MANIFEST_DIR is this
// crate's real root). Under `trybuild`, the synthetic crate it compiles each
// `tests/compile_fail/*.rs` case into never contains data files (only
// `Cargo.toml`/`Cargo.lock` plus a `[[bin]]` pointing at the original,
// absolute `.rs` path), so any relative file lookup there always fails with
// "No such file" regardless of what path is given — there's no way to reach
// the JSON-parse or conflicting-derivation branches through a `.stderr`
// snapshot. `idl_missing_file` covers the one branch trybuild *can*
// represent faithfully (a file that's genuinely absent, which is equally
// true in both worlds).
#[cfg(test)]
mod expand_error_tests {
    use super::*;

    #[test]
    fn bad_json_error_names_the_file_path() {
        let input = quote! { vault, "tests/idls/truncated.json" };
        let err = expand(input).expect_err("truncated JSON should fail to parse");
        let msg = err.to_string();
        assert!(
            msg.contains("invalid IDL JSON in") && msg.contains("truncated.json"),
            "expected the file path in the error, got: {msg}"
        );
    }

    #[test]
    fn conflicting_pda_derivation_names_the_account() {
        let input = quote! { conflict, "tests/idls/conflict.json" };
        let err = expand(input).expect_err("vault derives two different ways in this fixture");
        let msg = err.to_string();
        assert!(
            msg.contains("account `vault` derives differently across instructions"),
            "expected the conflicting-derivation message, got: {msg}"
        );
    }

    #[test]
    fn malformed_program_const_surfaces_as_an_error() {
        // A non-32-byte pda.program const is a malformed IDL; the macro refuses
        // it, naming the account and the actual length, rather than deriving a
        // wrong address or silently demoting.
        let input = quote! { malformed_program, "tests/idls/malformed_program.json" };
        let err = expand(input).expect_err("a 4-byte program const is not a program id");
        let msg = err.to_string();
        assert!(
            msg.contains("user_x") && msg.contains("4 bytes"),
            "expected the malformed-program message with account and length, got: {msg}"
        );
    }
}

#[cfg(test)]
mod expand_emit_tests {
    use super::*;

    // The two well-known program addresses the emitter hands an interface-style
    // token account as its default. Byte-exactness of these base58 strings is the
    // contract this test guards: a one-character typo would silently point every
    // token instruction at the wrong program.
    const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

    #[test]
    fn interface_token_accounts_default_to_well_known_programs() {
        let tokens = expand(quote! { token_flow, "tests/idls/token_flow.json" })
            .expect("token_flow.json emits");
        let out = tokens.to_string();
        let flat: String = out.chars().filter(|c| !c.is_whitespace()).collect();

        // A token account carrying no fixed address in the IDL defaults to the
        // real program, and the emitted base58 literals are byte-exact.
        assert!(
            out.contains(&format!("\"{TOKEN_PROGRAM}\"")),
            "token_program default must be the SPL Token program: {out}"
        );
        assert!(
            out.contains(&format!("\"{ASSOCIATED_TOKEN_PROGRAM}\"")),
            "associated_token_program default must be the ATA program: {out}"
        );

        // Both token accounts are bundle FIELDS the caller can override, not
        // injected constants. A field appears as `pub <name>: Pubkey`; an injected
        // account would instead surface its name as a string label inside
        // `injected_programs()`, which never happens here.
        assert!(
            flat.contains("pubtoken_program") && flat.contains("pubassociated_token_program"),
            "token accounts must be emitted as bundle fields: {out}"
        );
        assert!(
            !out.contains("\"token_program\"") && !out.contains("\"associated_token_program\""),
            "token accounts must not be injected: {out}"
        );

        // The `optional: true` account becomes `Option<Pubkey>` defaulting to `None`.
        assert!(
            out.contains("receipt"),
            "optional account must be present: {out}"
        );
        assert!(
            flat.contains("::core::option::Option<::anchor_lang::prelude::Pubkey>"),
            "optional account must be Option<Pubkey>: {out}"
        );
        assert!(
            flat.contains("::core::option::Option::None"),
            "optional account must default to None: {out}"
        );
    }

    // The ATA program's 32 bytes, as they appear in `ata.json`'s `pda.program`
    // const. `user_x` is an associated-token account: it derives under THIS
    // program, not the host `PROGRAM_ID`.
    const ATA_PROGRAM_BYTES: &str =
        "140u8,151u8,37u8,143u8,78u8,36u8,137u8,241u8,187u8,61u8,16u8,41u8,20u8,142u8,13u8,131u8,\
         11u8,90u8,19u8,153u8,218u8,255u8,16u8,132u8,4u8,142u8,123u8,216u8,219u8,233u8,248u8,89u8";

    #[test]
    fn const_program_pda_derives_under_that_program() {
        let tokens = expand(quote! { ata, "tests/idls/ata.json" }).expect("ata.json emits");
        let out = tokens.to_string();
        let flat: String = out.chars().filter(|c| !c.is_whitespace()).collect();

        // The foreign program is derived under via `new_from_array` of its exact
        // 32 bytes, never re-encoded through base58.
        assert!(
            flat.contains(&format!(
                "::anchor_lang::prelude::Pubkey::new_from_array([{ATA_PROGRAM_BYTES}])"
            )),
            "user_x must derive under the ATA program's raw bytes: {out}"
        );
    }

    #[test]
    fn demoted_bundle_fields_carry_a_reason_doc() {
        // `demoted_fields.json` carries an arg-seeded PDA (`escrow`), a
        // dotted-data-path PDA (`vault`), and a plain `token_program`
        // account: every generated bundle field must explain, in a doc
        // comment, why it's caller-supplied rather than derived.
        let tokens = expand(quote! { demoted_fields, "tests/idls/demoted_fields.json" })
            .expect("demoted_fields.json emits");
        let out = tokens.to_string();

        assert!(
            out.contains("this PDA's seeds reference instruction arg `seed`"),
            "escrow's doc must name the arg seed: {out}"
        );
        assert!(
            out.contains("this PDA's seeds reference account data (`config.owner`)"),
            "vault's doc must name the dotted data path: {out}"
        );
        assert!(
            out.contains("Defaults to the classic SPL Token program; override for Token-2022."),
            "token_program's doc must carry the well-known-default note: {out}"
        );
    }
}

/// `vault_x` -> `VaultX`, `lp_vault` -> `LpVault`, `mint` -> `Mint`.
fn pascal_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut capitalize_next = true;
    for ch in snake.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            out.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}
