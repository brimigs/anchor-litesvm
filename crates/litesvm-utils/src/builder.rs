//! Builder pattern for simplified test environment setup
//!
//! This module provides a fluent API for setting up test environments
//! with automatic program deployment and configuration.

use litesvm::LiteSVM;
use solana_program::pubkey::Pubkey;

/// Builder for creating a LiteSVM instance with programs pre-deployed
///
/// This provides a more ergonomic way to set up test environments compared to
/// manually creating LiteSVM instances and deploying programs.
///
/// # Example
/// ```ignore
/// use litesvm_utils::LiteSVMBuilder;
/// use solana_program::pubkey::Pubkey;
///
/// // Simple single program setup
/// let program_id = Pubkey::new_unique();
/// let program_bytes = include_bytes!("../target/deploy/my_program.so");
/// let mut svm = LiteSVMBuilder::new()
///     .deploy_program(program_id, program_bytes)
///     .build();
///
/// // Or use the convenience method for single program
/// let mut svm = LiteSVMBuilder::build_with_program(program_id, program_bytes);
/// ```
pub struct LiteSVMBuilder {
    svm: LiteSVM,
    programs: Vec<(Pubkey, Vec<u8>)>,
}

impl LiteSVMBuilder {
    /// Create a new test environment builder
    pub fn new() -> Self {
        Self {
            svm: LiteSVM::new(),
            programs: Vec::new(),
        }
    }

    /// Add a program to be deployed
    ///
    /// Programs are deployed in the order they are added.
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
        self.programs.push((program_id, program_bytes.to_vec()));
        self
    }

    /// Build the LiteSVM instance with all programs deployed
    ///
    /// # Returns
    ///
    /// Returns the configured LiteSVM instance with all programs deployed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut svm = builder.build();
    /// ```
    pub fn build(mut self) -> LiteSVM {
        // Deploy all programs
        for (program_id, program_bytes) in self.programs {
            self.svm.add_program(program_id, &program_bytes);
        }

        self.svm
    }

    /// Convenience method to quickly set up a single program
    ///
    /// This is equivalent to:
    /// ```ignore
    /// LiteSVMBuilder::new()
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
    /// Returns a configured LiteSVM instance with the program deployed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut svm = LiteSVMBuilder::build_with_program(program_id, program_bytes);
    /// ```
    pub fn build_with_program(program_id: Pubkey, program_bytes: &[u8]) -> LiteSVM {
        Self::new()
            .deploy_program(program_id, program_bytes)
            .build()
    }

    /// Convenience method to quickly set up multiple programs
    ///
    /// # Arguments
    ///
    /// * `programs` - A slice of (program_id, program_bytes) tuples
    ///
    /// # Returns
    ///
    /// Returns a configured LiteSVM instance with all programs deployed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let programs = vec![
    ///     (program_id1, program_bytes1),
    ///     (program_id2, program_bytes2),
    /// ];
    /// let mut svm = LiteSVMBuilder::build_with_programs(&programs);
    /// ```
    pub fn build_with_programs(programs: &[(Pubkey, &[u8])]) -> LiteSVM {
        let mut builder = Self::new();
        for (program_id, program_bytes) in programs {
            builder = builder.deploy_program(*program_id, program_bytes);
        }
        builder.build()
    }
}

impl Default for LiteSVMBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for LiteSVM to add program deployment capabilities
pub trait ProgramTestExt {
    /// Deploy a program to this LiteSVM instance
    ///
    /// # Example
    /// ```no_run
    /// # use litesvm_utils::ProgramTestExt;
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let mut svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// # let program_bytes = vec![];
    /// svm.deploy_program(program_id, &program_bytes);
    /// ```
    fn deploy_program(&mut self, program_id: Pubkey, program_bytes: &[u8]);
}

impl ProgramTestExt for LiteSVM {
    fn deploy_program(&mut self, program_id: Pubkey, program_bytes: &[u8]) {
        self.add_program(program_id, program_bytes);
    }
}