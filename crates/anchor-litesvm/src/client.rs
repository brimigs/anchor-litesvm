//! Integration with anchor-client for consistent production/testing syntax
//!
//! This module provides adapters to use anchor-client's Program interface
//! with LiteSVM as the backend, enabling the same syntax for both testing
//! and production code.

use litesvm::LiteSVM;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::rc::Rc;

/// A LiteSVM-backed client implementation for anchor-client
///
/// This adapter allows using anchor-client's Program interface with LiteSVM,
/// providing consistent syntax between testing and production environments.
///
/// # Example
/// ```ignore
/// use anchor_litesvm::LiteSvmClient;
/// use anchor_client::Program;
/// use litesvm::LiteSVM;
///
/// // Create LiteSVM instance
/// let mut svm = LiteSVM::new();
/// svm.add_program(program_id, program_bytes);
///
/// // Create client with LiteSVM backend
/// let payer = Keypair::new();
/// let client = LiteSvmClient::new(svm, payer);
///
/// // Use standard anchor-client Program interface
/// let program = client.program(program_id);
///
/// // Now use identical syntax to production!
/// let result = program
///     .request()
///     .accounts(my_program::accounts::Transfer {
///         from: from_account,
///         to: to_account,
///     })
///     .args(my_program::instruction::Transfer {
///         amount: 1000,
///     })
///     .send()?;
/// ```
pub struct LiteSvmClient {
    svm: Rc<LiteSVM>,
    payer: Keypair,
}

impl LiteSvmClient {
    /// Create a new LiteSVM-backed client
    ///
    /// # Arguments
    /// * `svm` - The LiteSVM instance to use as backend
    /// * `payer` - The default payer for transactions
    pub fn new(svm: LiteSVM, payer: Keypair) -> Self {
        Self {
            svm: Rc::new(svm),
            payer,
        }
    }

    /// Create a Program instance for the given program ID (placeholder)
    ///
    /// Note: Full implementation requires creating a custom RPC adapter for LiteSVM.
    /// This is a placeholder showing the intended API.
    ///
    /// # Arguments
    /// * `program_id` - The ID of the Anchor program
    ///
    /// # Example
    /// ```ignore
    /// // Future API when RPC adapter is implemented:
    /// let program = client.program(my_program_id);
    /// ```
    pub fn program(&self, _program_id: Pubkey) {
        // Note: This requires implementing a custom RPC client that bridges to LiteSVM
        // For now, we'll document this as the intended API
        // Full implementation would require creating a LiteSVM RPC adapter
        unimplemented!("LiteSVM RPC adapter not yet implemented. Use AnchorContext directly for now.")
    }

    /// Get a reference to the underlying LiteSVM instance
    pub fn svm(&self) -> &LiteSVM {
        &self.svm
    }

    /// Get the payer's public key
    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }
}

// TODO: Implement full RPC bridge for anchor-client
// This would involve:
// 1. Creating a custom RPC client that implements anchor_client's RPC traits
// 2. Bridging RPC calls to LiteSVM method calls
// 3. Handling account fetching, transaction sending, etc.
//
// Example structure:
// ```
// struct LiteSvmRpcClient {
//     svm: Rc<RefCell<LiteSVM>>,
// }
//
// impl anchor_client::RequestBuilder for LiteSvmRpcClient {
//     // Implementation that bridges to LiteSVM
// }
// ```

/// Builder for setting up anchor-client compatible testing
///
/// This provides a convenient way to set up testing with anchor-client syntax.
///
/// # Example
/// ```ignore
/// use anchor_litesvm::ClientBuilder;
///
/// let client = ClientBuilder::new()
///     .add_program(program_id, program_bytes)
///     .build();
///
/// let program = client.program(program_id);
/// ```
pub struct ClientBuilder {
    svm: LiteSVM,
    programs: Vec<(Pubkey, Vec<u8>)>,
    payer: Option<Keypair>,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self {
            svm: LiteSVM::new(),
            programs: Vec::new(),
            payer: None,
        }
    }

    /// Add a program to deploy
    pub fn add_program(mut self, program_id: Pubkey, program_bytes: &[u8]) -> Self {
        self.programs.push((program_id, program_bytes.to_vec()));
        self
    }

    /// Set the payer keypair
    pub fn with_payer(mut self, payer: Keypair) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Build the client with all programs deployed
    pub fn build(mut self) -> LiteSvmClient {
        // Deploy all programs
        for (program_id, program_bytes) in self.programs {
            self.svm.add_program(program_id, &program_bytes);
        }

        // Create payer if not provided
        let payer = self.payer.unwrap_or_else(Keypair::new);

        LiteSvmClient::new(self.svm, payer)
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let payer = Keypair::new();

        let client = ClientBuilder::new()
            .with_payer(payer.insecure_clone())
            .build();

        assert_eq!(client.payer(), payer.pubkey());
    }
}