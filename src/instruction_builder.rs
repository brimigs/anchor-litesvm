use crate::instruction::calculate_anchor_discriminator;
use crate::transaction::{TransactionError, TransactionResult};
use anchor_lang::AnchorSerialize;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;

/// Fluent builder for creating Anchor instructions with less boilerplate
///
/// This builder provides a more ergonomic API for constructing instructions,
/// handling account metadata, and managing signers automatically.
///
/// # Example
///
/// ```ignore
/// use anchor_litesvm::{AnchorContext, tuple_args};
/// use solana_sdk::signature::{Keypair, Signer};
///
/// let mut ctx = /* ... */;
/// let user = Keypair::new();
/// let account = Pubkey::new_unique();
///
/// let result = ctx.instruction_builder("transfer")
///     .signer("user", &user)
///     .account_mut("from", from_account)
///     .account_mut("to", to_account)
///     .token_program()
///     .args(tuple_args((amount,)))
///     .execute(&mut ctx, &[&user])?;
/// ```
pub struct InstructionBuilder {
    program_id: Pubkey,
    instruction_name: String,
    accounts: Vec<(String, AccountMeta)>,
    account_indices: HashMap<String, usize>,
    data: Vec<u8>,
}

impl InstructionBuilder {
    /// Create a new instruction builder
    ///
    /// # Arguments
    ///
    /// * `program_id` - The Anchor program ID
    /// * `instruction_name` - The name of the instruction (used for discriminator)
    pub fn new(program_id: &Pubkey, instruction_name: &str) -> Self {
        Self {
            program_id: *program_id,
            instruction_name: instruction_name.to_string(),
            accounts: Vec::new(),
            account_indices: HashMap::new(),
            data: Vec::new(),
        }
    }

    /// Add a read-only account
    ///
    /// # Arguments
    ///
    /// * `name` - A descriptive name for the account (for debugging)
    /// * `pubkey` - The public key of the account
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.account("mint", mint_pubkey)
    /// ```
    pub fn account(mut self, name: &str, pubkey: Pubkey) -> Self {
        let index = self.accounts.len();
        self.accounts.push((
            name.to_string(),
            AccountMeta::new_readonly(pubkey, false),
        ));
        self.account_indices.insert(name.to_string(), index);
        self
    }

    /// Add a writable account
    ///
    /// # Arguments
    ///
    /// * `name` - A descriptive name for the account (for debugging)
    /// * `pubkey` - The public key of the account
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.account_mut("token_account", token_account_pubkey)
    /// ```
    pub fn account_mut(mut self, name: &str, pubkey: Pubkey) -> Self {
        let index = self.accounts.len();
        self.accounts.push((
            name.to_string(),
            AccountMeta::new(pubkey, false),
        ));
        self.account_indices.insert(name.to_string(), index);
        self
    }

    /// Add a signer account (automatically marked as writable)
    ///
    /// # Arguments
    ///
    /// * `name` - A descriptive name for the account (for debugging)
    /// * `keypair` - The keypair that will sign the transaction
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.signer("user", &user_keypair)
    /// ```
    pub fn signer(mut self, name: &str, keypair: &Keypair) -> Self {
        let index = self.accounts.len();
        self.accounts.push((
            name.to_string(),
            AccountMeta::new(keypair.pubkey(), true),
        ));
        self.account_indices.insert(name.to_string(), index);
        self
    }

    /// Add a read-only signer account
    ///
    /// Use this for signers that don't need write access to their own account.
    ///
    /// # Arguments
    ///
    /// * `name` - A descriptive name for the account
    /// * `keypair` - The keypair that will sign the transaction
    pub fn signer_readonly(mut self, name: &str, keypair: &Keypair) -> Self {
        let index = self.accounts.len();
        self.accounts.push((
            name.to_string(),
            AccountMeta::new_readonly(keypair.pubkey(), true),
        ));
        self.account_indices.insert(name.to_string(), index);
        self
    }

    /// Add the system program
    ///
    /// Convenience method that adds the system program to the instruction accounts.
    pub fn system_program(self) -> Self {
        self.account("system_program", solana_program::system_program::id())
    }

    /// Add the token program
    ///
    /// Convenience method that adds the SPL Token program to the instruction accounts.
    pub fn token_program(self) -> Self {
        self.account("token_program", spl_token::id())
    }

    /// Add the associated token program
    ///
    /// Convenience method that adds the Associated Token Account program.
    pub fn associated_token_program(self) -> Self {
        self.account("associated_token_program", spl_associated_token_account::id())
    }

    /// Add the rent sysvar
    ///
    /// Convenience method that adds the rent sysvar account.
    pub fn rent_sysvar(self) -> Self {
        self.account("rent", solana_program::sysvar::rent::id())
    }

    /// Set instruction arguments using AnchorSerialize
    ///
    /// This method automatically calculates the discriminator and serializes the arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - The instruction arguments (use `tuple_args()` to avoid struct definitions)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use anchor_litesvm::tuple_args;
    ///
    /// builder.args(tuple_args((amount, recipient)))
    /// ```
    pub fn args<T: AnchorSerialize>(mut self, args: T) -> Self {
        let discriminator = calculate_anchor_discriminator(&self.instruction_name);
        self.data = discriminator.to_vec();
        args.serialize(&mut self.data)
            .expect("Failed to serialize instruction args");
        self
    }

    /// Build the instruction
    ///
    /// Finalizes the builder and returns a Solana instruction.
    ///
    /// # Returns
    ///
    /// Returns the built instruction or an error if args were not set.
    ///
    /// # Errors
    ///
    /// Returns an error if `.args()` was not called before building.
    pub fn build(self) -> Result<Instruction, Box<dyn std::error::Error>> {
        if self.data.is_empty() {
            return Err("No instruction data provided. Call .args() before .build()".into());
        }

        let accounts: Vec<AccountMeta> = self.accounts
            .into_iter()
            .map(|(_, meta)| meta)
            .collect();

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data: self.data,
        })
    }

    /// Get the account at a specific position (useful for debugging)
    pub fn get_account(&self, name: &str) -> Option<&AccountMeta> {
        self.account_indices
            .get(name)
            .and_then(|&index| self.accounts.get(index))
            .map(|(_, meta)| meta)
    }

    /// Get all accounts (useful for debugging)
    pub fn accounts(&self) -> Vec<&AccountMeta> {
        self.accounts.iter().map(|(_, meta)| meta).collect()
    }

    /// Build and execute the instruction with the given signers
    ///
    /// This is a convenience method when you have access to a mutable AnchorContext.
    /// Note: This requires passing the context and signers, as the builder doesn't hold them.
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, tuple_args};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let maker = Keypair::new();
    /// # let escrow_pda = Pubkey::new_unique();
    /// # let mint_a = Pubkey::new_unique();
    /// # let mint_b = Pubkey::new_unique();
    /// # let maker_ata_a = Pubkey::new_unique();
    /// # let vault = Pubkey::new_unique();
    /// let result = ctx.instruction_builder("make")
    ///     .signer("maker", &maker)
    ///     .account_mut("escrow", escrow_pda)
    ///     .account("mint_a", mint_a)
    ///     .account("mint_b", mint_b)
    ///     .account_mut("maker_ata_a", maker_ata_a)
    ///     .account_mut("vault", vault)
    ///     .system_program()
    ///     .args(tuple_args((42u64, 500u64, 1000u64)))
    ///     .execute(&mut ctx, &[&maker])
    ///     .unwrap();
    /// ```
    pub fn execute(
        self,
        ctx: &mut crate::AnchorContext,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError> {
        // Save the instruction name before consuming self
        let instruction_name = self.instruction_name.clone();

        let instruction = self
            .build()
            .map_err(|e| TransactionError::BuildError(e.to_string()))?;

        if signers.is_empty() {
            return Err(TransactionError::BuildError(
                "No signers provided".to_string(),
            ));
        }

        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&signers[0].pubkey()),
            signers,
            ctx.svm.latest_blockhash(),
        );

        match ctx.svm.send_transaction(tx) {
            Ok(result) => Ok(TransactionResult::new(
                result,
                Some(instruction_name),
            )),
            Err(e) => Err(TransactionError::ExecutionFailed(format!("{:?}", e))),
        }
    }
}

/// Wrapper type for tuple arguments to implement AnchorSerialize
///
/// This allows you to pass tuple arguments directly to instructions
/// without defining separate structs for each instruction.
///
/// # Example
///
/// ```ignore
/// use anchor_litesvm::tuple_args;
///
/// // Instead of defining a struct:
/// // struct TransferArgs { amount: u64, recipient: Pubkey }
///
/// // You can use tuple args:
/// builder.args(tuple_args((amount, recipient)))
/// ```
pub struct TupleArgs<T>(pub T);

// Manual implementation of AnchorSerialize for tuple wrappers
// Empty tuple for instructions with no arguments
impl AnchorSerialize for TupleArgs<()> {
    fn serialize<W: std::io::Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        // No data to serialize for empty tuple
        Ok(())
    }
}

impl<T1: AnchorSerialize> AnchorSerialize for TupleArgs<(T1,)> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.0.serialize(writer)
    }
}

impl<T1: AnchorSerialize, T2: AnchorSerialize> AnchorSerialize for TupleArgs<(T1, T2)> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.0.serialize(writer)?;
        self.0.1.serialize(writer)
    }
}

impl<T1: AnchorSerialize, T2: AnchorSerialize, T3: AnchorSerialize> AnchorSerialize for TupleArgs<(T1, T2, T3)> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.0.serialize(writer)?;
        self.0.1.serialize(writer)?;
        self.0.2.serialize(writer)
    }
}

impl<T1: AnchorSerialize, T2: AnchorSerialize, T3: AnchorSerialize, T4: AnchorSerialize> AnchorSerialize for TupleArgs<(T1, T2, T3, T4)> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.0.serialize(writer)?;
        self.0.1.serialize(writer)?;
        self.0.2.serialize(writer)?;
        self.0.3.serialize(writer)
    }
}

/// Convenience function to wrap tuples for serialization
///
/// This helper function creates a `TupleArgs` wrapper that implements `AnchorSerialize`
/// for tuple types, allowing you to pass instruction arguments without defining structs.
///
/// # Example
///
/// ```ignore
/// use anchor_litesvm::tuple_args;
///
/// // For no arguments
/// builder.args(tuple_args(()))
///
/// // For single argument
/// builder.args(tuple_args((amount,)))
///
/// // For multiple arguments
/// builder.args(tuple_args((amount, recipient, memo)))
/// ```
pub fn tuple_args<T>(args: T) -> TupleArgs<T> {
    TupleArgs(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;

    #[test]
    fn test_builder_basic() {
        let program_id = Pubkey::new_unique();
        let user = Keypair::new();
        let account = Pubkey::new_unique();

        #[derive(BorshSerialize)]
        struct TestArgs {
            value: u64,
        }

        impl AnchorSerialize for TestArgs {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                BorshSerialize::serialize(self, writer)
            }
        }

        let ix = InstructionBuilder::new(&program_id, "test")
            .signer("user", &user)
            .account_mut("account", account)
            .system_program()
            .args(TestArgs { value: 42 })
            .build()
            .unwrap();

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.data.len() >= 8); // At least discriminator
    }

    #[test]
    fn test_tuple_args() {
        let program_id = Pubkey::new_unique();
        let user = Keypair::new();

        // Test with tuple args - no struct needed!
        let ix = InstructionBuilder::new(&program_id, "test")
            .signer("user", &user)
            .args(tuple_args((42u64, 100u64, 200u64)))
            .build()
            .unwrap();

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 1);
        assert!(ix.data.len() >= 8 + 24); // discriminator + 3 u64s
    }

    #[test]
    fn test_account_ordering() {
        let program_id = Pubkey::new_unique();
        let user = Keypair::new();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();

        let builder = InstructionBuilder::new(&program_id, "test")
            .signer("user", &user)
            .account_mut("account1", account1)
            .account("account2", account2)
            .system_program();

        // Verify we can query accounts by name
        assert_eq!(builder.get_account("user").unwrap().pubkey, user.pubkey());
        assert_eq!(builder.get_account("account1").unwrap().pubkey, account1);
        assert_eq!(builder.get_account("system_program").unwrap().pubkey, solana_program::system_program::id());

        // Verify ordering is preserved
        let accounts = builder.accounts();
        assert_eq!(accounts[0].pubkey, user.pubkey());
        assert_eq!(accounts[1].pubkey, account1);
        assert_eq!(accounts[2].pubkey, account2);
    }
}