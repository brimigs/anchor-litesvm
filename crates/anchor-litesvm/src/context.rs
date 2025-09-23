use crate::account::AccountError;
#[allow(deprecated)]
use crate::instruction_builder::InstructionBuilder;
use anchor_client::Program;
use anchor_lang::AccountDeserialize;
use litesvm::LiteSVM;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::rc::Rc;
use litesvm_utils::TransactionResult;

/// Wrapper around LiteSVM that provides Anchor-specific utilities with anchor-client integration
///
/// This struct maintains a reference to a LiteSVM instance and provides
/// an anchor-client Program instance for IDL-based instruction generation,
/// enabling test code to use the same syntax as production client code.
pub struct AnchorContext {
    /// Direct access to the underlying LiteSVM instance
    pub svm: LiteSVM,
    /// The Anchor program ID
    pub program_id: Pubkey,
    /// The anchor-client Program instance for IDL-based operations
    program: Program<Rc<Keypair>>,
    /// The payer keypair
    payer: Rc<Keypair>,
}

impl AnchorContext {
    /// Create a new AnchorContext with an existing LiteSVM instance
    ///
    /// Note: This creates a default payer and funds it. For more control,
    /// use AnchorLiteSVM builder or new_with_program.
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
    pub fn new(mut svm: LiteSVM, program_id: Pubkey) -> Self {
        use anchor_client::{Client, Cluster};
        use solana_client::rpc_client::RpcClient;
        use solana_sdk::commitment_config::CommitmentConfig;

        // Create a default payer and fund it
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        let payer = Rc::new(payer);

        // Create a mock RPC client for anchor-client
        // This is sufficient for building instructions - we don't need actual RPC
        let _mock_rpc = RpcClient::new_mock("succeeds");

        // Create anchor-client instance with mock cluster
        let client = Client::new_with_options(
            Cluster::Custom(
                "http://127.0.0.1:8899".to_string(),
                "ws://127.0.0.1:8900".to_string(),
            ),
            payer.clone(),
            CommitmentConfig::confirmed(),
        );

        let program: Program<Rc<Keypair>> = client.program(program_id).unwrap();

        Self {
            svm,
            program_id,
            program,
            payer,
        }
    }

    /// Create a new AnchorContext with anchor-client components
    pub(crate) fn new_with_program(
        svm: LiteSVM,
        program_id: Pubkey,
        program: Program<Rc<Keypair>>,
        payer: Rc<Keypair>,
    ) -> Self {
        Self {
            svm,
            program_id,
            program,
            payer,
        }
    }

    /// Get the anchor-client Program instance for IDL-based instruction building
    ///
    /// This is the primary way to build instructions using the production-ready syntax
    /// that matches actual client code.
    ///
    /// # Example
    /// ```ignore
    /// let program = ctx.program();
    /// let ix = program
    ///     .request()
    ///     .accounts(my_program::accounts::MyInstruction { ... })
    ///     .args(my_program::instruction::MyArgs { ... })
    ///     .instructions()?[0];
    /// ```
    pub fn program(&self) -> &Program<Rc<Keypair>> {
        &self.program
    }

    /// Get a mutable reference to the anchor-client Program instance
    pub fn program_mut(&mut self) -> &mut Program<Rc<Keypair>> {
        &mut self.program
    }

    /// Get the payer keypair
    pub fn payer(&self) -> &Rc<Keypair> {
        &self.payer
    }

    /// Execute a single instruction using LiteSVM
    ///
    /// This is a convenience method for executing instructions generated
    /// by anchor-client's Program API.
    ///
    /// # Example
    /// ```ignore
    /// let ix = ctx.program()
    ///     .request()
    ///     .accounts(...)
    ///     .args(...)
    ///     .instructions()?[0];
    ///
    /// ctx.execute_instruction(ix, &[&signer])?;
    /// ```
    pub fn execute_instruction(
        &mut self,
        instruction: solana_program::instruction::Instruction,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, Box<dyn std::error::Error>> {
        // Determine the payer - use the first signer if provided, otherwise use the context's payer
        let payer_pubkey = if !signers.is_empty() {
            signers[0].pubkey()
        } else {
            self.payer.pubkey()
        };

        // Build and sign the transaction
        let tx = Transaction::new_signed_with_payer(
            &[instruction.clone()],
            Some(&payer_pubkey),
            signers,
            self.svm.latest_blockhash(),
        );

        // Execute the transaction
        match self.svm.send_transaction(tx) {
            Ok(result) => Ok(TransactionResult::new(
                result,
                Some(format!("instruction to {}", instruction.program_id)),
            )),
            Err(failed) => Ok(TransactionResult::new_failed(
                format!("{:?}", failed.err),
                failed.meta,
                Some(format!("instruction to {}", instruction.program_id)),
            )),
        }
    }

    /// Execute multiple instructions in a single transaction
    pub fn execute_instructions(
        &mut self,
        instructions: Vec<solana_program::instruction::Instruction>,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, Box<dyn std::error::Error>> {
        // Determine the payer
        let payer_pubkey = if !signers.is_empty() {
            signers[0].pubkey()
        } else {
            self.payer.pubkey()
        };

        // Build and sign the transaction
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer_pubkey),
            signers,
            self.svm.latest_blockhash(),
        );

        // Execute the transaction
        match self.svm.send_transaction(tx) {
            Ok(result) => Ok(TransactionResult::new(
                result,
                Some("batch transaction".to_string()),
            )),
            Err(failed) => Ok(TransactionResult::new_failed(
                format!("{:?}", failed.err),
                failed.meta,
                Some("batch transaction".to_string()),
            )),
        }
    }

    /// Send and confirm a transaction (convenience method)
    pub fn send_and_confirm_transaction(
        &mut self,
        transaction: &Transaction,
    ) -> Result<Signature, Box<dyn std::error::Error>> {
        match self.svm.send_transaction(transaction.clone()) {
            Ok(_) => Ok(transaction.signatures[0]),
            Err(e) => Err(format!("Transaction failed: {:?}", e).into()),
        }
    }

    /// Get an Anchor account from the blockchain
    ///
    /// This fetches and deserializes an Anchor account from the current state.
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorContext;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use anchor_lang::AccountDeserialize;
    /// # let svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// # let ctx = AnchorContext::new(svm, program_id);
    /// # struct MyAccount {}
    /// # impl AccountDeserialize for MyAccount {
    /// #     fn try_deserialize(buf: &mut &[u8]) -> Result<Self, anchor_lang::error::Error> {
    /// #         Ok(MyAccount {})
    /// #     }
    /// #     fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, anchor_lang::error::Error> {
    /// #         Ok(MyAccount {})
    /// #     }
    /// # }
    /// let account_pubkey = Pubkey::new_unique();
    /// let account: MyAccount = ctx.get_account(&account_pubkey).unwrap();
    /// ```
    pub fn get_account<T>(&self, address: &Pubkey) -> Result<T, AccountError>
    where
        T: AccountDeserialize,
    {
        let account_data = self.svm
            .get_account(address)
            .ok_or(AccountError::AccountNotFound(*address))?;

        // Deserialize the account data
        let mut data = account_data.data.as_slice();
        T::try_deserialize(&mut data).map_err(|e| AccountError::DeserializationError(e.to_string()))
    }

    /// Get an Anchor account without discriminator check
    ///
    /// Use this for accounts that don't have the standard Anchor discriminator.
    pub fn get_account_unchecked<T>(&self, address: &Pubkey) -> Result<T, AccountError>
    where
        T: AccountDeserialize,
    {
        let account_data = self.svm
            .get_account(address)
            .ok_or(AccountError::AccountNotFound(*address))?;

        // Deserialize the account data without discriminator check
        let mut data = account_data.data.as_slice();
        T::try_deserialize_unchecked(&mut data)
            .map_err(|e| AccountError::DeserializationError(e.to_string()))
    }

    /// Create a funded account (convenience method)
    pub fn create_funded_account(&mut self, lamports: u64) -> Result<Keypair, Box<dyn std::error::Error>> {
        let account = Keypair::new();
        self.svm.airdrop(&account.pubkey(), lamports)
            .map_err(|e| format!("Airdrop failed: {:?}", e))?;
        Ok(account)
    }

    /// Airdrop lamports to an account (convenience method)
    pub fn airdrop(&mut self, pubkey: &Pubkey, lamports: u64) -> Result<(), Box<dyn std::error::Error>> {
        self.svm.airdrop(pubkey, lamports)
            .map_err(|e| format!("Airdrop failed: {:?}", e))?;
        Ok(())
    }

    /// Get the latest blockhash
    pub fn latest_blockhash(&self) -> solana_sdk::hash::Hash {
        self.svm.latest_blockhash()
    }

    /// Check if an account exists
    pub fn account_exists(&self, pubkey: &Pubkey) -> bool {
        self.svm.get_account(pubkey).is_some()
    }

    /// Create a new instruction builder for this program (DEPRECATED)
    ///
    /// **DEPRECATED**: Use `ctx.program()` instead for IDL-based instruction building.
    ///
    /// Returns a fluent builder for constructing instructions with less boilerplate.
    /// This method is maintained for backward compatibility but new code should
    /// use the anchor-client Program API.
    ///
    /// # Example (Deprecated)
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, tuple_args};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// # let mut ctx = AnchorContext::new(svm, program_id);
    /// # let user = Keypair::new();
    /// # let from = Pubkey::new_unique();
    /// # let to = Pubkey::new_unique();
    /// let result = ctx.instruction_builder("transfer")
    ///     .signer("user", &user)
    ///     .account_mut("from", from)
    ///     .account_mut("to", to)
    ///     .args(tuple_args((100u64,)))
    ///     .execute(&mut ctx, &[&user])
    ///     .unwrap();
    /// ```
    #[deprecated(since="0.2.0", note="Use ctx.program() for IDL-based, production-ready instruction building")]
    #[allow(deprecated)]
    pub fn instruction_builder(&self, instruction_name: &str) -> InstructionBuilder {
        InstructionBuilder::new(&self.program_id, instruction_name)
    }
}