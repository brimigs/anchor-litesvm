//! The proc-macro half of `anchor_litesvm`: [`bundles_from_idl!`] generates
//! per-instruction pubkey bundles from a committed Anchor IDL, paired against
//! the `client::{accounts, args}` types `declare_program!` emits from the
//! same file. Re-exported from `anchor_litesvm`; depend on that crate, not
//! this one.

pub(crate) mod classify;
pub(crate) mod idl;
mod idl_bundles;

use proc_macro::TokenStream;

/// Generate per-instruction pubkey bundles from a committed Anchor IDL,
/// against the `client::{accounts, args}` types that
/// `anchor_lang::declare_program!` generates from the same file. Invoke both
/// in the same module: `bundles_from_idl!(vault)` reads `idls/vault.json`
/// (relative to the crate root, the same convention `declare_program!`
/// follows) and targets the adjacent `vault::client` module.
///
/// For each instruction `deposit`, it emits a `DepositBundle` struct (one
/// `Pubkey` field per account the caller must supply), its `Default`, a
/// `From<DepositBundle> for vault::client::accounts::Deposit` that derives the
/// instruction's PDAs and injects its fixed addresses, an
/// `impl BuildableIx<DepositBundle> for vault::client::args::Deposit`, and a
/// `<account>_pda(...)` helper per derivable PDA. One module-level
/// `injected_programs()` lists every fixed address the IDL pins.
///
/// A second argument overrides the IDL path:
/// `bundles_from_idl!(vault, "fixtures/vault.json")`.
///
/// # Foreign-program PDAs
///
/// A PDA usually derives under the IDL's own program, but Anchor emits a
/// `pda.program` for every `associated_token::` account: an ATA derives under
/// the associated-token program, not the program the IDL describes. Both the
/// `From` impl and the `<account>_pda` helper honor that program (the derivation
/// runs under the const program's raw bytes, or under another account when the
/// IDL names one). A `pda.program` the macro can't resolve to a pubkey at build
/// time (an instruction arg, an account-data path, or a const that isn't a
/// 32-byte program id) demotes the account to a caller-supplied bundle field,
/// the same rule an unresolvable seed follows.
///
/// # One program per module
///
/// `PROGRAM_ID` and `injected_programs()` are free items emitted at module
/// scope, so two `bundles_from_idl!` invocations in the same module collide
/// (`E0428: the name `PROGRAM_ID` is defined multiple times`). This mirrors
/// `declare_program!`, which emits its own module-level items per invocation
/// too. Testing two programs in the same file, wrap each pair of
/// `declare_program!` + `bundles_from_idl!` calls in its own `mod`:
///
/// ```ignore
/// mod escrow_ixs {
///     anchor_lang::declare_program!(escrow);
///     anchor_litesvm::bundles_from_idl!(escrow);
/// }
/// mod vault_ixs {
///     anchor_lang::declare_program!(vault);
///     anchor_litesvm::bundles_from_idl!(vault);
/// }
/// ```
#[proc_macro]
pub fn bundles_from_idl(input: TokenStream) -> TokenStream {
    match idl_bundles::expand(input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
