//! An error-code-to-name registry, the failure-path twin of
//! `InstructionNames`.
//!
//! ## Why this exists
//!
//! When a program returns `ProgramError::Custom(n)`, the runtime logs
//! `Program <id> failed: custom program error: 0x<n>` and nothing more: the
//! number, not the name. An Anchor program additionally emits an
//! `AnchorError ... Error Code: <Name>` log line, so its failures already
//! read as `EscrowExpired` in the raw logs. A raw Pinocchio program emits only
//! the bare code, so its failures render as `custom program error: 0x7`, and
//! `assert_error` / `send_err_named("InvalidAmount")` can't match a name that
//! never appears.
//!
//! This registry closes that gap the same way the instruction registry does:
//! the test registers `code -> name` for its program, the table rides on the
//! `TransactionResult`, and its `assert_error` resolves the top-level
//! instruction's failing custom code through it. The resolved name then lets
//! `assert_error` / `send_err_named` match by name even when only the bare
//! code appeared in the logs.
//!
//! It is consulted only alongside the raw logs and error field, so a program
//! that already emits its own error name keeps matching on that; the registry
//! is an additional match source for programs that emit only the code.

use solana_program::pubkey::Pubkey;
use std::collections::HashMap;

/// A per-program table of `custom-error-code -> name`. Attach it to a
/// `TransactionResult` via
/// [`with_error_names`](crate::transaction::TransactionResult::with_error_names),
/// or register through the `AnchorContext` helpers, which thread it onto
/// every send.
#[derive(Clone, Default, Debug)]
pub struct ErrorNames {
    /// Keyed by the program's base58 id, then by the `ProgramError::Custom`
    /// code. A program's error enum is small, so a flat per-program map is
    /// plenty.
    by_program: HashMap<String, HashMap<u32, String>>,
}

impl ErrorNames {
    /// An empty registry: every lookup misses, so failures render exactly as
    /// before (the raw `custom program error: 0x..`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `code -> name` for `program_id`, where `code` is the value the
    /// program passes to `ProgramError::Custom`. Chainable.
    pub fn register(
        &mut self,
        program_id: Pubkey,
        code: u32,
        name: impl Into<String>,
    ) -> &mut Self {
        self.by_program
            .entry(program_id.to_string())
            .or_default()
            .insert(code, name.into());
        self
    }

    /// Resolve a program's custom error code to its registered name, or `None`.
    pub fn resolve(&self, program_id: &str, code: u32) -> Option<&str> {
        self.by_program
            .get(program_id)?
            .get(&code)
            .map(String::as_str)
    }

    /// True when nothing has been registered. The model build skips the
    /// failure-name lookup entirely in this case.
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
    fn resolves_registered_code() {
        let mut errors = ErrorNames::new();
        errors.register(pid(), 7, "InvalidAmount");
        errors.register(pid(), 0, "InvalidInstruction");
        let s = pid().to_string();
        assert_eq!(errors.resolve(&s, 7), Some("InvalidAmount"));
        assert_eq!(errors.resolve(&s, 0), Some("InvalidInstruction"));
        assert_eq!(errors.resolve(&s, 99), None);
    }

    #[test]
    fn unknown_program_misses() {
        let mut errors = ErrorNames::new();
        errors.register(pid(), 7, "InvalidAmount");
        let other = Pubkey::new_from_array([8u8; 32]).to_string();
        assert_eq!(errors.resolve(&other, 7), None);
    }
}
