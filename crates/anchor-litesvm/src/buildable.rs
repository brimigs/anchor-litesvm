//! Type-level pairing of an instruction's args struct with its accounts struct.
//!
//! [`BuildableIx`] lets [`Program::build_ix`] accept just an args struct plus a
//! caller-supplied bundle of pubkeys, then construct the corresponding
//! accounts struct via [`From`]. Implementing it for each instruction's args
//! struct (one line per instruction) gives the program a one-method ix
//! constructor with compile-time-checked accounts/args pairing.
//!
//! # Example
//!
//! Given a program you've already declared with `declare_program!`:
//!
//! ```ignore
//! anchor_lang::declare_program!(escrow);
//! anchor_litesvm::bundles_from_idl!(escrow);
//! ```
//!
//! `bundles_from_idl!` reads the same IDL `declare_program!` does and, for
//! each instruction, emits a `<Ix>Bundle` struct (one field per account,
//! `Pubkey`-typed) plus `From<<Ix>Bundle> for escrow::client::accounts::<Ix>`
//! and `BuildableIx<<Ix>Bundle> for escrow::client::args::<Ix>`. Accounts the
//! macro can infer from the IDL (the system program, a well-known token
//! program) are auto-injected rather than added as bundle fields.
//!
//! In your test:
//!
//! ```ignore
//! let ix = ctx.program().build_ix(
//!     DepositBundle { user, vault_state, vault },
//!     instruction::Deposit { amount: 1_000_000 },
//! );
//! ```
//!
//! The bundle type parameter is on the trait (not an associated type) so the
//! same args struct can implement `BuildableIx<TestBundle>` and
//! `BuildableIx<ProdBundle>` if a program wants more than one bundle shape.
//! For a program without a shippable IDL, or a shape `bundles_from_idl!`
//! can't infer, hand-write `impl BuildableIx<YourBundle> for instruction::Foo`
//! directly; the macro is a convenience, not the only way to satisfy the trait.
//!
//! For negative-path tests that need a deliberately-wrong account, see
//! [`Program::build_ix_with`], which takes a closure with `&mut Self::Accounts`.
//!
//! [`Program::build_ix`]: crate::program::Program::build_ix
//! [`Program::build_ix_with`]: crate::program::Program::build_ix_with

use anchor_lang::{InstructionData, ToAccountMetas};

/// Pairs an instruction's args struct with its accounts struct, given a
/// caller-supplied bundle that knows how to construct the accounts.
///
/// The bundle type `B` is whatever pubkey container the program author finds
/// convenient (a single struct, a tuple, etc.). The projection itself
/// (bundle → accounts) is expressed via `B: Into<Self::Accounts>` on the
/// call sites that need it ([`Program::build_ix`],
/// [`Program::build_ix_with`], and the [`Tx`] fluent builder), rather
/// than as a bound on the associated type. That keeps this trait open to any
/// source that emits either `From<B> for Self::Accounts` (in which case
/// `Into` is implied for free, via the blanket impl) or
/// `Into<Self::Accounts> for B` directly. `bundles_from_idl!` emits the
/// former.
///
/// See the [module-level docs](crate::buildable) for an end-to-end example.
///
/// [`Program::build_ix`]: crate::program::Program::build_ix
/// [`Program::build_ix_with`]: crate::program::Program::build_ix_with
/// [`Tx`]: crate::tx::Tx
#[diagnostic::on_unimplemented(
    message = "`{Self}` can't be built with bundle `{B}`: no `BuildableIx<{B}>` impl",
    label = "no `BuildableIx<{B}>` for this args type",
    note = "Generate the pairing with `bundles_from_idl!(<program>)` next to \
            your `declare_program!(<program>)`, or hand-write \
            `impl BuildableIx<{B}> for {Self}`.",
    note = "If the bundle came from `bundles_from_idl!`, double-check that \
            you're passing the bundle generated for this instruction \
            (`<Ix>Bundle`), not a sibling's."
)]
pub trait BuildableIx<B>: InstructionData {
    /// The accounts struct paired with this instruction's args struct.
    type Accounts: ToAccountMetas;
}
