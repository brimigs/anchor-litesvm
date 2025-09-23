//! Builder pattern for setting up Anchor test environments
//!
//! This module provides builders specifically designed for Anchor programs,
//! extending the base LiteSVM builder functionality.

use crate::AnchorContext;
use litesvm_utils::LiteSVMBuilder;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};

/// Builder for creating an AnchorContext with programs pre-deployed
///
/// This provides a more ergonomic way to set up Anchor test environments.
///
/// # Example
/// ```ignore
/// use anchor_litesvm::AnchorLiteSVM;
/// use solana_program::pubkey::Pubkey;
///
/// // Simple single program setup
/// let program_id = Pubkey::new_unique();
/// let program_bytes = include_bytes!("../target/deploy/my_program.so");
/// let mut ctx = AnchorLiteSVM::new()
///     .deploy_program(program_id, program_bytes)
///     .build();
///
/// // Or use the convenience method for single program
/// let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
///
/// // Build instructions using the native builder
/// let ix = ctx.instruction()
///     .accounts(...)
///     .args(...)
///     .build();
/// ```
pub struct AnchorLiteSVM {
    svm_builder: LiteSVMBuilder,
    primary_program_id: Option<Pubkey>,
    payer: Option<Keypair>,
}

impl AnchorLiteSVM {
    /// Create a new Anchor test environment builder
    pub fn new() -> Self {
        Self {
            svm_builder: LiteSVMBuilder::new(),
            primary_program_id: None,
            payer: None,
        }
    }

    /// Set the payer keypair for transactions
    ///
    /// If not set, a new keypair will be generated and funded.
    pub fn with_payer(mut self, payer: Keypair) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Add a program to be deployed
    ///
    /// The first program added becomes the primary program for the AnchorContext.
    ///
    /// # Arguments
    ///
    /// * `program_id` - The program ID to deploy at
    /// * `program_bytes` - The compiled program bytes (.so file contents)
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.deploy_program(program_id, program_bytes)
    /// ```
    pub fn deploy_program(mut self, program_id: Pubkey, program_bytes: &[u8]) -> Self {
        // Set the first program as primary if not already set
        if self.primary_program_id.is_none() {
            self.primary_program_id = Some(program_id);
        }

        self.svm_builder = self.svm_builder.deploy_program(program_id, program_bytes);
        self
    }

    /// Build the AnchorContext with all programs deployed
    ///
    /// # Returns
    ///
    /// Returns an AnchorContext with the primary program ID and deployed programs
    ///
    /// # Panics
    ///
    /// Panics if no programs were added
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut ctx = builder.build();
    /// ```
    pub fn build(self) -> AnchorContext {
        let program_id = self.primary_program_id
            .expect("No programs added. Call deploy_program() at least once.");

        let mut svm = self.svm_builder.build();

        // Create or use provided payer
        let payer = self.payer.unwrap_or_else(|| {
            let payer = Keypair::new();
            // Fund the payer account
            svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
            payer
        });

        AnchorContext::new_with_payer(svm, program_id, payer)
    }

    /// Convenience method to quickly set up a single Anchor program
    ///
    /// This is equivalent to:
    /// ```ignore
    /// AnchorLiteSVM::new()
    ///     .deploy_program(program_id, program_bytes)
    ///     .build()
    /// ```
    ///
    /// # Arguments
    ///
    /// * `program_id` - The program ID to deploy at
    /// * `program_bytes` - The compiled program bytes
    ///
    /// # Returns
    ///
    /// Returns an AnchorContext with the program deployed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
    /// ```
    pub fn build_with_program(program_id: Pubkey, program_bytes: &[u8]) -> AnchorContext {
        Self::new()
            .deploy_program(program_id, program_bytes)
            .build()
    }

    /// Convenience method to set up multiple programs
    ///
    /// The first program in the list becomes the primary program.
    ///
    /// # Arguments
    ///
    /// * `programs` - A slice of (program_id, program_bytes) tuples
    ///
    /// # Returns
    ///
    /// Returns an AnchorContext with all programs deployed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let programs = vec![
    ///     (program_id1, program_bytes1),
    ///     (program_id2, program_bytes2),
    /// ];
    /// let mut ctx = AnchorLiteSVM::build_with_programs(&programs);
    /// ```
    pub fn build_with_programs(programs: &[(Pubkey, &[u8])]) -> AnchorContext {
        let mut builder = Self::new();
        for (program_id, program_bytes) in programs {
            builder = builder.deploy_program(*program_id, program_bytes);
        }
        builder.build()
    }
}

impl Default for AnchorLiteSVM {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for AnchorContext to provide program deployment
pub trait ProgramTestExt {
    /// Deploy an additional program to this context
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, ProgramTestExt};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// # let mut ctx = AnchorContext::new(svm, program_id);
    /// # let other_program_id = Pubkey::new_unique();
    /// # let other_program_bytes = vec![];
    /// ctx.deploy_program(other_program_id, &other_program_bytes);
    /// ```
    fn deploy_program(&mut self, program_id: Pubkey, program_bytes: &[u8]);
}

impl ProgramTestExt for AnchorContext {
    fn deploy_program(&mut self, program_id: Pubkey, program_bytes: &[u8]) {
        self.svm.add_program(program_id, program_bytes);
    }
}