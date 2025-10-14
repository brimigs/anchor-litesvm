//! Transaction execution and result handling utilities
//!
//! This module provides convenient wrappers for executing transactions
//! and handling their results in tests.

use litesvm::types::TransactionMetadata;
use litesvm::LiteSVM;
use solana_program::instruction::Instruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Transaction execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Transaction build error: {0}")]
    BuildError(String),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
}

/// Wrapper around LiteSVM's TransactionMetadata with helper methods for testing
///
/// This struct provides convenient methods for analyzing transaction results,
/// including log inspection, compute unit tracking, and success assertions.
///
/// # Example
///
/// ```ignore
/// let result = svm.send_instruction(ix, &[&signer])?;
/// result.assert_success();
/// assert!(result.has_log("Transfer complete"));
/// println!("Used {} compute units", result.compute_units());
/// ```
pub struct TransactionResult {
    inner: TransactionMetadata,
    instruction_name: Option<String>,
    error: Option<String>,
}

impl TransactionResult {
    /// Create a new TransactionResult wrapper for successful transaction
    ///
    /// # Arguments
    ///
    /// * `result` - The transaction metadata from LiteSVM
    /// * `instruction_name` - Optional name of the instruction for debugging
    pub fn new(result: TransactionMetadata, instruction_name: Option<String>) -> Self {
        Self {
            inner: result,
            instruction_name,
            error: None,
        }
    }

    /// Create a new TransactionResult wrapper for failed transaction
    ///
    /// # Arguments
    ///
    /// * `error` - The error message
    /// * `result` - The transaction metadata from LiteSVM
    /// * `instruction_name` - Optional name of the instruction for debugging
    pub fn new_failed(error: String, result: TransactionMetadata, instruction_name: Option<String>) -> Self {
        Self {
            inner: result,
            instruction_name,
            error: Some(error),
        }
    }

    /// Assert that the transaction succeeded, panic with logs if it failed
    ///
    /// # Returns
    ///
    /// Returns self for chaining
    ///
    /// # Example
    ///
    /// ```ignore
    /// result.assert_success();
    /// ```
    pub fn assert_success(&self) -> &Self {
        assert!(
            self.error.is_none(),
            "Transaction failed: {}\nLogs:\n{}",
            self.error.as_ref().unwrap_or(&"Unknown error".to_string()),
            self.logs().join("\n")
        );
        self
    }

    /// Check if the transaction succeeded
    ///
    /// # Returns
    ///
    /// true if the transaction succeeded, false otherwise
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Get the error message if the transaction failed
    ///
    /// # Returns
    ///
    /// The error message if the transaction failed, None otherwise
    pub fn error(&self) -> Option<&String> {
        self.error.as_ref()
    }

    /// Get the transaction logs
    ///
    /// # Returns
    ///
    /// A slice of log messages
    pub fn logs(&self) -> &[String] {
        &self.inner.logs
    }

    /// Check if the logs contain a specific message
    ///
    /// # Arguments
    ///
    /// * `message` - The message to search for
    ///
    /// # Returns
    ///
    /// true if the message is found in the logs, false otherwise
    pub fn has_log(&self, message: &str) -> bool {
        self.inner.logs.iter().any(|log| log.contains(message))
    }

    /// Find a log entry containing the specified text
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern to search for
    ///
    /// # Returns
    ///
    /// The first matching log entry, or None
    pub fn find_log(&self, pattern: &str) -> Option<&String> {
        self.inner.logs.iter().find(|log| log.contains(pattern))
    }

    /// Get the compute units consumed
    ///
    /// # Returns
    ///
    /// The number of compute units consumed
    pub fn compute_units(&self) -> u64 {
        self.inner.compute_units_consumed
    }

    /// Print the transaction logs
    pub fn print_logs(&self) {
        println!("=== Transaction Logs ===");
        if let Some(name) = &self.instruction_name {
            println!("Instruction: {}", name);
        }
        for log in &self.inner.logs {
            println!("{}", log);
        }
        if let Some(err) = &self.error {
            println!("Error: {}", err);
        }
        println!("Compute Units: {}", self.compute_units());
        println!("========================");
    }

    /// Get the inner TransactionMetadata for direct access
    pub fn inner(&self) -> &TransactionMetadata {
        &self.inner
    }

    /// Assert that the transaction failed
    ///
    /// # Panics
    ///
    /// Panics if the transaction succeeded
    ///
    /// # Returns
    ///
    /// Returns self for chaining
    ///
    /// # Example
    ///
    /// ```ignore
    /// result.assert_failure();
    /// ```
    pub fn assert_failure(&self) -> &Self {
        assert!(
            self.error.is_some(),
            "Expected transaction to fail, but it succeeded.\nLogs:\n{}",
            self.logs().join("\n")
        );
        self
    }

    /// Assert that the transaction failed with a specific error message
    ///
    /// # Arguments
    ///
    /// * `expected_error` - The expected error message (substring match)
    ///
    /// # Panics
    ///
    /// Panics if the transaction succeeded or failed with a different error
    ///
    /// # Returns
    ///
    /// Returns self for chaining
    ///
    /// # Example
    ///
    /// ```ignore
    /// result.assert_error("insufficient funds");
    /// ```
    pub fn assert_error(&self, expected_error: &str) -> &Self {
        match &self.error {
            Some(error) => {
                assert!(
                    error.contains(expected_error),
                    "Transaction failed with unexpected error.\nExpected substring: {}\nActual error: {}\nLogs:\n{}",
                    expected_error,
                    error,
                    self.logs().join("\n")
                );
            }
            None => {
                panic!(
                    "Expected transaction to fail with error containing '{}', but it succeeded.\nLogs:\n{}",
                    expected_error,
                    self.logs().join("\n")
                );
            }
        }
        self
    }

    /// Assert that the transaction failed with a specific error code
    ///
    /// This is useful for asserting Anchor custom errors.
    ///
    /// # Arguments
    ///
    /// * `error_code` - The expected error code number
    ///
    /// # Panics
    ///
    /// Panics if the transaction succeeded or failed with a different error code
    ///
    /// # Returns
    ///
    /// Returns self for chaining
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Assert that transaction failed with custom error code 6000
    /// result.assert_error_code(6000);
    /// ```
    pub fn assert_error_code(&self, error_code: u32) -> &Self {
        let error_code_str = format!("custom program error: 0x{:x}", error_code);
        self.assert_error(&error_code_str)
    }

    /// Assert that the transaction failed with a specific Anchor error
    ///
    /// This checks for Anchor's error code format in the logs.
    ///
    /// # Arguments
    ///
    /// * `error_name` - The name of the Anchor error
    ///
    /// # Panics
    ///
    /// Panics if the transaction succeeded or the error wasn't found in logs
    ///
    /// # Returns
    ///
    /// Returns self for chaining
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Assert that transaction failed with Anchor error
    /// result.assert_anchor_error("InsufficientFunds");
    /// ```
    pub fn assert_anchor_error(&self, error_name: &str) -> &Self {
        self.assert_failure();

        // Check if error name appears in logs
        let found_in_logs = self.logs().iter().any(|log| log.contains(error_name));

        // Also check the error message
        let found_in_error = self.error
            .as_ref()
            .map(|e| e.contains(error_name))
            .unwrap_or(false);

        assert!(
            found_in_logs || found_in_error,
            "Expected Anchor error '{}' not found in transaction logs or error message.\nError: {:?}\nLogs:\n{}",
            error_name,
            self.error,
            self.logs().join("\n")
        );
        self
    }

    /// Assert that the logs contain a specific error message
    ///
    /// Unlike `assert_error`, this only checks the logs, not the error field.
    ///
    /// # Arguments
    ///
    /// * `error_message` - The expected error message in logs
    ///
    /// # Panics
    ///
    /// Panics if the error message is not found in logs
    ///
    /// # Returns
    ///
    /// Returns self for chaining
    ///
    /// # Example
    ///
    /// ```ignore
    /// result.assert_log_error("Transfer amount exceeds balance");
    /// ```
    pub fn assert_log_error(&self, error_message: &str) -> &Self {
        assert!(
            self.has_log(error_message),
            "Expected error message '{}' not found in logs.\nLogs:\n{}",
            error_message,
            self.logs().join("\n")
        );
        self
    }
}

impl fmt::Debug for TransactionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransactionResult")
            .field("instruction", &self.instruction_name)
            .field("success", &self.is_success())
            .field("error", &self.error())
            .field("compute_units", &self.compute_units())
            .field("log_count", &self.logs().len())
            .finish()
    }
}

/// Transaction helper methods for LiteSVM
pub trait TransactionHelpers {
    /// Send a single instruction and return a wrapped result
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TransactionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let signer = Keypair::new();
    /// let result = svm.send_instruction(ix, &[&signer]).unwrap();
    /// result.assert_success();
    /// ```
    fn send_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError>;

    /// Send multiple instructions in a single transaction
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TransactionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix1 = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let ix2 = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let signer = Keypair::new();
    /// let result = svm.send_instructions(&[ix1, ix2], &[&signer]).unwrap();
    /// result.assert_success();
    /// ```
    fn send_instructions(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError>;

    /// Send a transaction and return a wrapped result
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::TransactionHelpers;
    /// # use litesvm::LiteSVM;
    /// # use solana_sdk::transaction::Transaction;
    /// # use solana_program::instruction::Instruction;
    /// # use solana_sdk::signature::{Keypair, Signer};
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let signer = Keypair::new();
    /// let tx = Transaction::new_signed_with_payer(
    ///     &[ix],
    ///     Some(&signer.pubkey()),
    ///     &[&signer],
    ///     svm.latest_blockhash(),
    /// );
    /// let result = svm.send_transaction_result(tx).unwrap();
    /// result.assert_success();
    /// ```
    fn send_transaction_result(
        &mut self,
        transaction: Transaction,
    ) -> Result<TransactionResult, TransactionError>;
}

impl TransactionHelpers for LiteSVM {
    fn send_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError> {
        if signers.is_empty() {
            return Err(TransactionError::BuildError("No signers provided".to_string()));
        }

        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&signers[0].pubkey()),
            signers,
            self.latest_blockhash(),
        );

        self.send_transaction_result(tx)
    }

    fn send_instructions(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError> {
        if signers.is_empty() {
            return Err(TransactionError::BuildError("No signers provided".to_string()));
        }

        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&signers[0].pubkey()),
            signers,
            self.latest_blockhash(),
        );

        self.send_transaction_result(tx)
    }

    fn send_transaction_result(
        &mut self,
        transaction: Transaction,
    ) -> Result<TransactionResult, TransactionError> {
        match self.send_transaction(transaction) {
            Ok(result) => Ok(TransactionResult::new(result, None)),
            Err(failed) => {
                // Return a failed transaction result with metadata
                Ok(TransactionResult::new_failed(
                    format!("{:?}", failed.err),
                    failed.meta,
                    None,
                ))
            }
        }
    }
}