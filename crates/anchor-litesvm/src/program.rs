//! Native implementation of anchor-client's Program API without RPC dependencies.
//!
//! This module provides a production-compatible syntax that matches anchor-client
//! but works directly with LiteSVM without any network overhead.

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// A mock Program struct that mimics anchor-client's Program API
/// but works natively with LiteSVM without RPC connections.
///
/// This provides the same syntax as production code:
/// ```ignore
/// let ix = program
///     .request()
///     .accounts(...)
///     .args(...)
///     .instructions()?[0];
/// ```
pub struct Program {
    program_id: Pubkey,
}

impl Program {
    /// Create a new Program instance for the given program ID
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }

    /// Start building a request, matching anchor-client's syntax
    pub fn request(&self) -> RequestBuilder {
        RequestBuilder::new(self.program_id)
    }

    /// Get the program ID
    pub fn id(&self) -> Pubkey {
        self.program_id
    }
}

/// Builder for constructing requests, matching anchor-client's RequestBuilder API
pub struct RequestBuilder {
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
}

impl RequestBuilder {
    /// Create a new request builder
    fn new(program_id: Pubkey) -> Self {
        Self {
            program_id,
            accounts: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Set the accounts for this instruction
    ///
    /// Matches anchor-client's syntax exactly:
    /// ```ignore
    /// .accounts(my_program::accounts::MyInstruction { ... })
    /// ```
    pub fn accounts<T: ToAccountMetas>(mut self, accounts: T) -> Self {
        self.accounts = accounts.to_account_metas(None);
        self
    }

    /// Set the instruction arguments
    ///
    /// Matches anchor-client's syntax exactly:
    /// ```ignore
    /// .args(my_program::instruction::MyArgs { ... })
    /// ```
    pub fn args<T: InstructionData>(mut self, args: T) -> Self {
        self.data = args.data();
        self
    }

    /// Build the instructions, returning a Result with a Vec to match anchor-client
    ///
    /// This returns `Result<Vec<Instruction>>` to match anchor-client's API exactly.
    /// In production, multiple instructions might be needed (e.g., for compute budget),
    /// but in tests we typically just need one, hence the common pattern of `.instructions()?[0]`
    pub fn instructions(self) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
        if self.data.is_empty() {
            return Err("No instruction data provided. Call .args() before .instructions()".into());
        }

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: self.accounts,
            data: self.data,
        };

        Ok(vec![instruction])
    }

    /// Alternative method that returns a single instruction directly
    ///
    /// This is a convenience method not in anchor-client, but useful for tests
    /// where you know there's only one instruction.
    pub fn instruction(self) -> Result<Instruction, Box<dyn std::error::Error>> {
        self.instructions().map(|mut ixs| ixs.remove(0))
    }
}

/// Type alias to match anchor-client's Program<Rc<Keypair>> pattern
///
/// While we don't actually need the Keypair generic, keeping it
/// maintains syntax compatibility with anchor-client.
pub type MockProgram = Program;

#[cfg(test)]
mod tests {
    use super::{Program, RequestBuilder};
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
    fn test_production_compatible_syntax() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let account = Pubkey::new_unique();

        // This syntax exactly matches anchor-client!
        let program = Program::new(program_id);
        let ix = program
            .request()
            .accounts(TestAccounts { user, account })
            .args(TestArgs { amount: 100 })
            .instructions()
            .unwrap()
            .remove(0); // Common pattern in anchor-client usage

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.data.len() > 8);
    }

    #[test]
    fn test_convenience_method() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let account = Pubkey::new_unique();

        let program = Program::new(program_id);
        let ix = program
            .request()
            .accounts(TestAccounts { user, account })
            .args(TestArgs { amount: 100 })
            .instruction() // Convenience method for single instruction
            .unwrap();

        assert_eq!(ix.program_id, program_id);
    }
}