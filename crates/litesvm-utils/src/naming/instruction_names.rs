//! A discriminator-to-name registry, so a program without an IDL (and so
//! without the `Program log: Instruction: <Name>` line Anchor emits) can still
//! render its instructions by name in every view.
//!
//! ## Why this exists
//!
//! The CPI model resolves an instruction's name through
//! `decode_instruction`: built-in tables
//! cover System / SPL Token / Associated-Token, and Anchor programs get their
//! names from the log line the framework emits. A raw Pinocchio program has
//! neither: its instruction is a one-byte discriminator with hand-packed args,
//! and nothing in the logs spells out "Make". So the renderers fall back to the
//! program alias, and a tree reads `escrow ✓` three times instead of
//! `escrow::Make` / `escrow::Take` / `escrow::Cancel`.
//!
//! This registry closes that gap. The test registers `discriminator -> name`
//! for its program, the registry rides along on the
//! `TransactionResult` (like the alias table does),
//! and the model consults it as the last resort after the built-in tables and
//! the log line. It is the bottom of the resolution stack: anything the runtime
//! or a built-in decoder already named wins, so registering a name never
//! shadows a more authoritative one.
//!
//! ## Discriminator shape
//!
//! A discriminator is a byte *prefix* matched against the head of the
//! instruction's data. Pinocchio's one-byte tag is the prefix `[0]`; an
//! eight-byte Anchor discriminator is the prefix of those eight bytes; a custom
//! scheme is whatever leading bytes identify the variant. When several
//! registered prefixes match (a `[0]` and a longer `[0, 1]`, say), the longest
//! one wins, so a coarse catch-all never shadows a specific entry.

use solana_program::pubkey::Pubkey;
use std::collections::HashMap;

/// A per-program table of `discriminator-prefix -> instruction name`. Attach it
/// to a `TransactionResult` via
/// [`with_instruction_names`](crate::transaction::TransactionResult::with_instruction_names),
/// or register through the `AnchorContext` helpers, which thread it onto every
/// send automatically. See the [module docs](self) for the resolution order.
#[derive(Clone, Default, Debug)]
pub struct InstructionNames {
    /// Keyed by the program's base58 id (the same string form
    /// `decode_instruction` matches on), so
    /// the lookup is a direct string compare with no `Pubkey` parse at render
    /// time. The value is insertion-ordered; ties are broken by prefix length
    /// at resolve time, not insertion order.
    by_program: HashMap<String, Vec<(Vec<u8>, String)>>,
}

impl InstructionNames {
    /// An empty registry: every lookup misses, so the renderers behave exactly
    /// as they did before any registration (program-alias fallback).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `discriminator -> name` for `program_id`. The discriminator is a
    /// byte prefix (see the [module docs](self)); pass a `[u8; N]`, a
    /// `&[u8]`, or a `Vec<u8>`. Chainable.
    pub fn register(
        &mut self,
        program_id: Pubkey,
        discriminator: impl Into<Vec<u8>>,
        name: impl Into<String>,
    ) -> &mut Self {
        self.by_program
            .entry(program_id.to_string())
            .or_default()
            .push((discriminator.into(), name.into()));
        self
    }

    /// Register a one-byte discriminator: the common case for Pinocchio and
    /// other hand-rolled programs whose instruction tag is `data[0]`. Sugar for
    /// [`register`](Self::register) with a single-byte prefix.
    pub fn register_byte(
        &mut self,
        program_id: Pubkey,
        discriminator: u8,
        name: impl Into<String>,
    ) -> &mut Self {
        self.register(program_id, [discriminator], name)
    }

    /// Resolve an instruction's name from its program id (base58) and data, or
    /// `None` when no registered prefix matches. Longest matching prefix wins.
    pub fn resolve(&self, program_id: &str, data: &[u8]) -> Option<&str> {
        let table = self.by_program.get(program_id)?;
        table
            .iter()
            .filter(|(disc, _)| data.starts_with(disc))
            .max_by_key(|(disc, _)| disc.len())
            .map(|(_, name)| name.as_str())
    }

    /// True when nothing has been registered. The model build skips the
    /// registry lookup entirely in this case (the overwhelmingly common path
    /// for Anchor tests, which never register).
    pub fn is_empty(&self) -> bool {
        self.by_program.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid() -> Pubkey {
        Pubkey::new_from_array([7u8; 32])
    }

    #[test]
    fn resolves_one_byte_discriminator_ignoring_trailing_args() {
        let mut names = InstructionNames::new();
        names.register_byte(pid(), 0, "Make");
        names.register_byte(pid(), 1, "Take");
        names.register_byte(pid(), 2, "Cancel");

        let s = pid().to_string();
        // Make carries 24 bytes of args after the tag; the prefix still matches.
        assert_eq!(
            names.resolve(&s, &[0, 7, 0, 0, 0, 0, 0, 0, 0]),
            Some("Make")
        );
        assert_eq!(names.resolve(&s, &[1]), Some("Take"));
        assert_eq!(names.resolve(&s, &[2]), Some("Cancel"));
        assert_eq!(names.resolve(&s, &[3]), None);
        assert_eq!(names.resolve(&s, &[]), None);
    }

    #[test]
    fn longest_matching_prefix_wins() {
        let mut names = InstructionNames::new();
        names.register(pid(), [0u8], "Generic");
        names.register(pid(), [0u8, 9u8], "Specific");
        let s = pid().to_string();
        assert_eq!(names.resolve(&s, &[0, 9, 1, 2]), Some("Specific"));
        assert_eq!(names.resolve(&s, &[0, 1]), Some("Generic"));
    }

    #[test]
    fn unknown_program_misses() {
        let mut names = InstructionNames::new();
        names.register_byte(pid(), 0, "Make");
        let other = Pubkey::new_from_array([8u8; 32]).to_string();
        assert_eq!(names.resolve(&other, &[0]), None);
    }
}
