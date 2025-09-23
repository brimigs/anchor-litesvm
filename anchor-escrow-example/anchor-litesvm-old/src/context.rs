use crate::account::{get_anchor_account, get_anchor_account_unchecked, AccountError};
use crate::instruction::build_anchor_instruction;
use crate::instruction_builder::InstructionBuilder;
use anchor_lang::{AccountDeserialize, AnchorSerialize};
use litesvm::LiteSVM;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

/// Wrapper around LiteSVM that provides Anchor-specific utilities
///
/// This struct maintains a reference to a LiteSVM instance and provides
/// convenience methods for working with Anchor programs while still
/// allowing direct access to all LiteSVM functionality.
pub struct AnchorContext {
    /// Direct access to the underlying LiteSVM instance
    pub svm: LiteSVM,
    /// The Anchor program ID for instruction building
    pub program_id: Pubkey,
}

impl AnchorContext {
    /// Create a new AnchorContext with an existing LiteSVM instance
    ///
    /// # Example
    /// ```no_run
    /// use litesvm::LiteSVM;
    /// use anchor_litesvm::AnchorContext;
    /// use solana_program::pubkey::Pubkey;
    ///
    /// let mut svm = LiteSVM::new();
    /// let program_id = Pubkey::new_unique();
    /// let ctx = AnchorContext::new(svm, program_id);
    /// ```
    pub fn new(svm: LiteSVM, program_id: Pubkey) -> Self {
        Self { svm, program_id }
    }

    /// Build an Anchor instruction with automatic discriminator calculation
    ///
    /// This method handles:
    /// - Calculating the 8-byte discriminator from the instruction name
    /// - Serializing the instruction arguments using Borsh
    /// - Creating a properly formatted Solana instruction
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorContext;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_program::instruction::AccountMeta;
    /// # use anchor_lang::AnchorSerialize;
    /// # use borsh::BorshSerialize;
    /// # #[derive(BorshSerialize)]
    /// # struct MakeArgs { seed: u64, amount: u64 }
    /// # impl AnchorSerialize for MakeArgs {
    /// #     fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
    /// #         BorshSerialize::serialize(self, writer)
    /// #     }
    /// # }
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let accounts = vec![
    ///     AccountMeta::new(Pubkey::new_unique(), true),
    ///     AccountMeta::new(Pubkey::new_unique(), false),
    /// ];
    /// let args = MakeArgs { seed: 42, amount: 1000 };
    /// let ix = ctx.build_instruction("make", accounts, args).unwrap();
    /// ```
    pub fn build_instruction<T>(
        &self,
        instruction_name: &str,
        accounts: Vec<AccountMeta>,
        args: T,
    ) -> Result<Instruction, Box<dyn std::error::Error>>
    where
        T: AnchorSerialize,
    {
        build_anchor_instruction(&self.program_id, instruction_name, accounts, args)
    }

    /// Fetch and deserialize an Anchor account
    ///
    /// This method retrieves an account from LiteSVM and deserializes it
    /// using Anchor's AccountDeserialize trait, properly handling the
    /// 8-byte discriminator.
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorContext;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use anchor_lang::{AccountDeserialize, error::{Error, ErrorCode}};
    /// # use borsh::{BorshSerialize, BorshDeserialize};
    /// #
    /// # // Define a simple account structure that implements AccountDeserialize
    /// # #[derive(BorshSerialize, BorshDeserialize)]
    /// # struct EscrowState {
    /// #     discriminator: [u8; 8],
    /// #     amount: u64,
    /// # }
    /// #
    /// # impl AccountDeserialize for EscrowState {
    /// #     fn try_deserialize(buf: &mut &[u8]) -> Result<Self, Error> {
    /// #         Self::try_deserialize_unchecked(buf)
    /// #     }
    /// #     fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, Error> {
    /// #         borsh::BorshDeserialize::deserialize(buf)
    /// #             .map_err(|_| ErrorCode::AccountDidNotDeserialize.into())
    /// #     }
    /// # }
    /// #
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let escrow_pda = Pubkey::new_unique();
    /// // This would retrieve and deserialize the account from LiteSVM
    /// // let escrow: EscrowState = ctx.get_anchor_account(&escrow_pda).unwrap();
    /// ```
    pub fn get_anchor_account<T>(&self, address: &Pubkey) -> Result<T, AccountError>
    where
        T: AccountDeserialize,
    {
        get_anchor_account(&self.svm, address)
    }

    /// Fetch and deserialize an Anchor account without discriminator verification
    ///
    /// Use this for accounts that don't have the standard Anchor discriminator.
    /// This skips the first 8 bytes and deserializes the remaining data.
    pub fn get_anchor_account_unchecked<T>(&self, address: &Pubkey) -> Result<T, AccountError>
    where
        T: borsh::BorshDeserialize,
    {
        get_anchor_account_unchecked(&self.svm, address)
    }

    /// Calculate a program-derived address (PDA)
    ///
    /// Convenience method for PDA calculation using the context's program ID
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorContext;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let user = Pubkey::new_unique();
    /// let seed = 42u64;
    /// let (pda, bump) = ctx.find_pda(&[
    ///     b"escrow",
    ///     user.as_ref(),
    ///     &seed.to_le_bytes(),
    /// ]);
    /// ```
    pub fn find_pda(&self, seeds: &[&[u8]]) -> (Pubkey, u8) {
        Pubkey::find_program_address(seeds, &self.program_id)
    }

    /// Update the program ID for this context
    ///
    /// Useful when testing multiple programs with the same LiteSVM instance
    pub fn set_program_id(&mut self, program_id: Pubkey) {
        self.program_id = program_id;
    }

    /// Create a new instruction builder with fluent API
    ///
    /// This provides a more ergonomic way to build instructions compared to
    /// manually creating AccountMeta vectors.
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorContext;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// let user = Keypair::new();
    /// let account = Pubkey::new_unique();
    ///
    /// let ix = ctx.instruction_builder("initialize")
    ///     .signer("user", &user)
    ///     .account_mut("account", account)
    ///     .system_program()
    ///     .args((42u64, 100u64))  // Tuple args - no struct needed!
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn instruction_builder(&self, instruction_name: &str) -> InstructionBuilder {
        InstructionBuilder::new(&self.program_id, instruction_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let ctx = AnchorContext::new(svm, program_id);
        assert_eq!(ctx.program_id, program_id);
    }

    #[test]
    fn test_pda_calculation() {
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let ctx = AnchorContext::new(svm, program_id);

        let user = Pubkey::new_unique();
        let (pda, bump) = ctx.find_pda(&[b"test", user.as_ref()]);

        // Verify it matches direct calculation
        let (expected_pda, expected_bump) =
            Pubkey::find_program_address(&[b"test", user.as_ref()], &program_id);

        assert_eq!(pda, expected_pda);
        assert_eq!(bump, expected_bump);
    }
}