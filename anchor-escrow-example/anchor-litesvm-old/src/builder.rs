//! Builder pattern for simplified test environment setup
//!
//! This module provides a fluent API for setting up test environments
//! with automatic program deployment and configuration.

use crate::AnchorContext;
use litesvm::LiteSVM;
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;

/// Builder for creating an AnchorContext with programs pre-deployed
///
/// This provides a more ergonomic way to set up test environments compared to
/// manually creating LiteSVM instances and deploying programs.
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
/// ```
pub struct AnchorLiteSVM {
    svm: LiteSVM,
    #[cfg(test)]
    pub(crate) programs: Vec<(Pubkey, Vec<u8>)>,
    #[cfg(not(test))]
    programs: Vec<(Pubkey, Vec<u8>)>,
    #[cfg(test)]
    pub(crate) primary_program_id: Option<Pubkey>,
    #[cfg(not(test))]
    primary_program_id: Option<Pubkey>,
}

impl AnchorLiteSVM {
    /// Create a new test environment builder
    pub fn new() -> Self {
        Self {
            svm: LiteSVM::new(),
            programs: Vec::new(),
            primary_program_id: None,
        }
    }

    /// Deploy a program to the test environment
    ///
    /// The first program deployed becomes the primary program for the AnchorContext.
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorLiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let program_id = Pubkey::new_unique();
    /// # let program_bytes = &[];
    /// let mut ctx = AnchorLiteSVM::new()
    ///     .deploy_program(program_id, program_bytes)
    ///     .build();
    /// ```
    pub fn deploy_program(mut self, program_id: Pubkey, program_bytes: &[u8]) -> Self {
        if self.primary_program_id.is_none() {
            self.primary_program_id = Some(program_id);
        }
        self.programs.push((program_id, program_bytes.to_vec()));
        self
    }

    /// Deploy multiple programs at once
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorLiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// let programs = vec![
    ///     (Pubkey::new_unique(), vec![]),
    ///     (Pubkey::new_unique(), vec![]),
    /// ];
    /// let mut ctx = AnchorLiteSVM::new()
    ///     .deploy_programs(programs)
    ///     .build();
    /// ```
    pub fn deploy_programs(mut self, programs: Vec<(Pubkey, Vec<u8>)>) -> Self {
        if self.primary_program_id.is_none() && !programs.is_empty() {
            self.primary_program_id = Some(programs[0].0);
        }
        self.programs.extend(programs);
        self
    }

    /// Set the primary program ID for the AnchorContext
    ///
    /// By default, the first deployed program becomes the primary program.
    /// Use this to override that behavior.
    pub fn with_primary_program(mut self, program_id: Pubkey) -> Self {
        self.primary_program_id = Some(program_id);
        self
    }

    /// Build the AnchorContext with all deployed programs
    ///
    /// # Panics
    /// Panics if no programs have been deployed
    pub fn build(mut self) -> AnchorContext {
        assert!(
            !self.programs.is_empty(),
            "At least one program must be deployed"
        );

        let primary_program_id = self
            .primary_program_id
            .expect("Primary program ID should be set");

        // Deploy all programs
        for (program_id, program_bytes) in self.programs {
            self.svm.add_program(program_id, &program_bytes);
        }

        AnchorContext::new(self.svm, primary_program_id)
    }

    /// Convenience method to create and build with a single program
    ///
    /// This is the most common use case and provides the simplest API.
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorLiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let program_id = Pubkey::new_unique();
    /// # let program_bytes = &[];
    /// // One line setup!
    /// let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);
    /// ```
    pub fn build_with_program(program_id: Pubkey, program_bytes: &[u8]) -> AnchorContext {
        Self::new()
            .deploy_program(program_id, program_bytes)
            .build()
    }

    /// Create a test environment with programs loaded from a map
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::AnchorLiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use std::collections::HashMap;
    /// let mut programs = HashMap::new();
    /// programs.insert(Pubkey::new_unique(), vec![]);
    /// programs.insert(Pubkey::new_unique(), vec![]);
    ///
    /// let mut ctx = AnchorLiteSVM::from_programs(programs, None);
    /// ```
    pub fn from_programs(
        programs: HashMap<Pubkey, Vec<u8>>,
        primary_program_id: Option<Pubkey>,
    ) -> AnchorContext {
        let mut builder = Self::new();

        // Convert HashMap to Vec
        let programs_vec: Vec<(Pubkey, Vec<u8>)> = programs.into_iter().collect();

        if let Some(primary) = primary_program_id {
            builder = builder.with_primary_program(primary);
        }

        builder.deploy_programs(programs_vec).build()
    }
}

impl Default for AnchorLiteSVM {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for even simpler initialization when you have the program ID as a constant
pub trait ProgramTestExt {
    /// Build test environment with this program
    ///
    /// # Example
    /// ```ignore
    /// use anchor_litesvm::ProgramTestExt;
    /// use solana_program::pubkey::Pubkey;
    ///
    /// const PROGRAM_ID: Pubkey = Pubkey::new_from_array([0; 32]);
    /// let program_bytes = include_bytes!("../target/deploy/program.so");
    ///
    /// // Using the extension trait
    /// let mut ctx = PROGRAM_ID.test_with(program_bytes);
    /// ```
    fn test_with(self, program_bytes: &[u8]) -> AnchorContext;
}

impl ProgramTestExt for Pubkey {
    fn test_with(self, program_bytes: &[u8]) -> AnchorContext {
        AnchorLiteSVM::build_with_program(self, program_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_single_program() {
        let program_id = Pubkey::new_unique();
        let program_bytes = vec![0u8; 100];

        let builder = AnchorLiteSVM::new()
            .deploy_program(program_id, &program_bytes);

        // Test that the builder stores the program correctly
        assert_eq!(builder.programs.len(), 1);
        assert_eq!(builder.primary_program_id, Some(program_id));
    }

    #[test]
    fn test_multiple_programs() {
        let program1_id = Pubkey::new_unique();
        let program2_id = Pubkey::new_unique();
        let program1_bytes = vec![0u8; 100];
        let program2_bytes = vec![0u8; 200];

        let builder = AnchorLiteSVM::new()
            .deploy_program(program1_id, &program1_bytes)
            .deploy_program(program2_id, &program2_bytes);

        // Test that both programs are stored
        assert_eq!(builder.programs.len(), 2);
        // First program should be primary by default
        assert_eq!(builder.primary_program_id, Some(program1_id));
    }

    #[test]
    fn test_with_primary_program() {
        let program1_id = Pubkey::new_unique();
        let program2_id = Pubkey::new_unique();
        let program1_bytes = vec![0u8; 100];
        let program2_bytes = vec![0u8; 200];

        let builder = AnchorLiteSVM::new()
            .deploy_program(program1_id, &program1_bytes)
            .deploy_program(program2_id, &program2_bytes)
            .with_primary_program(program2_id);

        // Test that primary program can be overridden
        assert_eq!(builder.primary_program_id, Some(program2_id));
    }

    #[test]
    #[should_panic(expected = "At least one program must be deployed")]
    fn test_build_without_programs() {
        // This should panic because no programs were deployed
        AnchorLiteSVM::new().build();
    }
}