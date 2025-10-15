//! Simplified instruction builder for LiteSVM testing without RPC overhead.
//!
//! This module provides a clean, testing-focused API that removes unnecessary
//! RPC-layer abstractions like `.request()` and `.remove(0)`.

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// A lightweight Program wrapper for building instructions in tests.
///
/// Simplified API for testing without RPC layer abstractions:
/// ```ignore
/// let ix = ctx.program()
///     .accounts(my_program::accounts::Transfer { ... })
///     .args(my_program::instruction::Transfer { ... })
///     .instruction()?;
/// ```
#[derive(Copy, Clone)]
pub struct Program {
    program_id: Pubkey,
}

impl Program {
    /// Create a new Program instance for the given program ID
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }

    /// Start building an instruction with accounts.
    ///
    /// This returns an `InstructionBuilder` that you can chain with `.args()` and `.instruction()`.
    ///
    /// # Example
    /// ```ignore
    /// let ix = ctx.program()
    ///     .accounts(my_program::accounts::Initialize {
    ///         user: user.pubkey(),
    ///         account: data_account,
    ///         system_program: system_program::id(),
    ///     })
    ///     .args(my_program::instruction::Initialize { value: 42 })
    ///     .instruction()?;
    /// ```
    pub fn accounts<T: ToAccountMetas>(self, accounts: T) -> InstructionBuilder {
        InstructionBuilder {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: Vec::new(),
        }
    }

    /// Get the program ID
    pub fn id(&self) -> Pubkey {
        self.program_id
    }
}

/// Builder for constructing instructions in a fluent, chainable manner.
///
/// You typically don't create this directly - use `program().accounts()` instead.
pub struct InstructionBuilder {
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
}

impl InstructionBuilder {
    /// Set the instruction arguments
    ///
    /// # Example
    /// ```ignore
    /// .args(my_program::instruction::Transfer { amount: 1000 })
    /// ```
    pub fn args<T: InstructionData>(mut self, args: T) -> Self {
        self.data = args.data();
        self
    }

    /// Build and return the instruction.
    ///
    /// This is the final method in the chain that produces the `Instruction`.
    ///
    /// # Example
    /// ```ignore
    /// let ix = ctx.program()
    ///     .accounts(...)
    ///     .args(...)
    ///     .instruction()?;
    /// ```
    pub fn instruction(self) -> Result<Instruction, Box<dyn std::error::Error>> {
        if self.data.is_empty() {
            return Err("No instruction data provided. Call .args() before .instruction()".into());
        }

        Ok(Instruction {
            program_id: self.program_id,
            accounts: self.accounts,
            data: self.data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Program;
    use anchor_lang::{prelude::*, InstructionData, ToAccountMetas};
    use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

    struct TestAccounts {
        user: Pubkey,
        account: Pubkey,
    }

    impl ToAccountMetas for TestAccounts {
        fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
            vec![
                AccountMeta::new(self.user, true),
                AccountMeta::new(self.account, false),
            ]
        }
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    struct TestArgs {
        amount: u64,
    }

    impl anchor_lang::Discriminator for TestArgs {
        const DISCRIMINATOR: &'static [u8] = &[1, 2, 3, 4, 5, 6, 7, 8];
    }

    impl InstructionData for TestArgs {
        fn data(&self) -> Vec<u8> {
            let mut data = Vec::new();
            data.extend_from_slice(Self::DISCRIMINATOR);
            self.serialize(&mut data).unwrap();
            data
        }
    }

    #[test]
    fn test_simplified_syntax() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let account = Pubkey::new_unique();

        // New simplified syntax for testing
        let program = Program::new(program_id);
        let ix = program
            .accounts(TestAccounts { user, account })
            .args(TestArgs { amount: 100 })
            .instruction()
            .unwrap();

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.data.len() > 8);
    }
}
