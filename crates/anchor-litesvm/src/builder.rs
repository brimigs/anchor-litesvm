//! Builder pattern for setting up Anchor test environments
//!
//! This module provides builders specifically designed for Anchor programs,
//! extending the base LiteSVM builder functionality.

use crate::AnchorContext;
use anchor_litesvm_compat::{Keypair, Signer};
use litesvm_utils::LiteSVMBuilder;
use solana_program::pubkey::Pubkey;

/// Builder for creating an [`AnchorContext`] with programs pre-deployed.
///
/// Every program deployed via this builder is **named** at the same call
/// site: the name registers as an alias in the resulting context's
/// alias table, so a failing send's printed logs read `escrow` instead of
/// the raw program ID. This makes the alias the default — no
/// `.alias(program::ID, "...")` ceremony after build.
///
/// # Example
///
/// ```ignore
/// use anchor_litesvm::AnchorLiteSVM;
/// use solana_program::pubkey::Pubkey;
///
/// // Single program: chained form.
/// let program_id = Pubkey::new_unique();
/// let program_bytes = include_bytes!("../target/deploy/my_program.so");
/// let mut ctx = AnchorLiteSVM::new()
///     .deploy_program(program_id, "my_program", program_bytes)
///     .build();
///
/// // Single program: one-shot.
/// let mut ctx = AnchorLiteSVM::build_with_program(program_id, "my_program", program_bytes);
/// ```
pub struct AnchorLiteSVM {
    svm_builder: LiteSVMBuilder,
    primary_program_id: Option<Pubkey>,
    payer: Option<Keypair>,
    /// `(pubkey, name)` pairs recorded by [`Self::deploy_program`]; installed
    /// as aliases on the [`AnchorContext`] at [`Self::build`] time so printed
    /// logs use the friendly names.
    program_aliases: Vec<(Pubkey, String)>,
}

impl AnchorLiteSVM {
    /// Create a new Anchor test environment builder
    pub fn new() -> Self {
        Self {
            svm_builder: LiteSVMBuilder::new(),
            primary_program_id: None,
            payer: None,
            program_aliases: Vec::new(),
        }
    }

    /// Set the payer keypair for transactions
    ///
    /// If not set, a new keypair will be generated and funded.
    pub fn with_payer(mut self, payer: Keypair) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Pin the world's starting slot, overriding the engine's default.
    ///
    /// Forwards to [`LiteSVMBuilder::start_slot`]; the pin is applied before the
    /// programs deploy so they stay visible at the chosen slot. Use it for tests
    /// whose determinism should not ride litesvm's version-varying default slot.
    pub fn start_slot(mut self, slot: u64) -> Self {
        self.svm_builder = self.svm_builder.start_slot(slot);
        self
    }

    /// Add a program to be deployed, registering its `name` as a pubkey
    /// alias so a failing send's printed logs name it instead of its raw id.
    ///
    /// The first program added becomes the primary program for the
    /// [`AnchorContext`]. The name is recorded now and installed on the
    /// context's alias table at [`Self::build`] time; a later
    /// `ctx.alias(id, "...")` override will shadow it (last write wins).
    ///
    /// # Arguments
    ///
    /// * `program_id` - The program ID to deploy at
    /// * `name` - Friendly label used in printed logs (e.g. `"escrow"`)
    /// * `program_bytes` - The compiled program bytes (.so file contents)
    pub fn deploy_program(
        mut self,
        program_id: Pubkey,
        name: impl Into<String>,
        program_bytes: &[u8],
    ) -> Self {
        // Set the first program as primary if not already set
        if self.primary_program_id.is_none() {
            self.primary_program_id = Some(program_id);
        }

        self.svm_builder = self.svm_builder.deploy_program(program_id, program_bytes);
        self.program_aliases.push((program_id, name.into()));
        self
    }

    /// Build the AnchorContext with all programs deployed
    ///
    /// # Returns
    ///
    /// Returns an [`AnchorContext`] with the primary program ID, all
    /// deployed programs, and their names installed as pubkey aliases.
    ///
    /// # Panics
    ///
    /// Panics if no programs were added.
    pub fn build(self) -> AnchorContext {
        let program_id = self
            .primary_program_id
            .expect("No programs added. Call deploy_program() at least once.");

        let mut svm = self.svm_builder.build();

        // Create or use provided payer
        let payer = self.payer.unwrap_or_else(|| {
            let payer = Keypair::new();
            // Fund the payer account
            svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
            payer
        });

        let mut ctx = AnchorContext::new_with_payer(svm, program_id, payer);
        for (pk, name) in self.program_aliases {
            ctx.alias(pk, name);
        }
        ctx
    }

    /// Convenience method to quickly set up a single named Anchor program
    ///
    /// This is equivalent to:
    /// ```ignore
    /// AnchorLiteSVM::new()
    ///     .deploy_program(program_id, name, program_bytes)
    ///     .build()
    /// ```
    pub fn build_with_program(
        program_id: Pubkey,
        name: impl Into<String>,
        program_bytes: &[u8],
    ) -> AnchorContext {
        Self::new()
            .deploy_program(program_id, name, program_bytes)
            .build()
    }

    /// Like [`build_with_program`](Self::build_with_program), but reads the
    /// program from a `.so` file at *runtime* instead of taking bytes embedded
    /// at compile time with `include_bytes!`. Reach for it when the artifact is
    /// produced by a separate build step (a plain `anchor build`) and may not
    /// exist when the test crate compiles, so an `include_bytes!` would not even
    /// build the test.
    ///
    /// Panics with a diagnosis when the file is missing or is a stub: an ELF
    /// under 4 KiB almost certainly built without its entrypoint (a
    /// feature-gated `entrypoint!` plus a plain `cargo build-sbf` yields an
    /// ~896-byte shell that fails to load as `EntrypointOutOfBounds`), so the
    /// panic names that rather than surfacing an opaque loader error later.
    ///
    /// ```ignore
    /// let mut ctx = AnchorLiteSVM::build_with_program_from_file(
    ///     staking::ID,
    ///     "staking",
    ///     "../../target/deploy/staking.so",
    /// );
    /// ```
    pub fn build_with_program_from_file(
        program_id: Pubkey,
        name: impl Into<String>,
        path: &str,
    ) -> AnchorContext {
        let bytes = std::fs::read(path).unwrap_or_else(|e| {
            panic!("build_with_program_from_file: read {path}: {e} (build the program first)")
        });
        assert!(
            bytes.len() >= 4096,
            "build_with_program_from_file: {path} is {} bytes — likely an \
             entrypoint-less stub; check the program's build features \
             (e.g. `--features sbf`)",
            bytes.len()
        );
        Self::new().deploy_program(program_id, name, &bytes).build()
    }

    /// Convenience method to set up multiple named programs
    ///
    /// The first program in the list becomes the primary program.
    ///
    /// # Arguments
    ///
    /// * `programs` - A slice of `(program_id, name, program_bytes)` tuples
    ///
    /// # Example
    ///
    /// ```ignore
    /// let programs = [
    ///     (program_id1, "amm", AMM_BYTES),
    ///     (program_id2, "mpl_core", MPL_CORE_BYTES),
    /// ];
    /// let mut ctx = AnchorLiteSVM::build_with_programs(&programs);
    /// ```
    pub fn build_with_programs(programs: &[(Pubkey, &str, &[u8])]) -> AnchorContext {
        let mut builder = Self::new();
        for (program_id, name, program_bytes) in programs {
            builder = builder.deploy_program(*program_id, *name, program_bytes);
        }
        builder.build()
    }
}

impl Default for AnchorLiteSVM {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for [`AnchorContext`] to deploy additional programs
/// after the context already exists (e.g. when a test needs a secondary
/// program loaded mid-scenario).
pub trait ProgramTestExt {
    /// Deploy an additional program to this context and register its
    /// `name` as a pubkey alias.
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, ProgramTestExt};
    /// # use anchor_litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # let svm = LiteSVM::new();
    /// # let program_id = Pubkey::new_unique();
    /// # let mut ctx = AnchorContext::new(svm, program_id);
    /// # let other_program_id = Pubkey::new_unique();
    /// # let other_program_bytes = vec![];
    /// ctx.deploy_program(other_program_id, "mpl_core", &other_program_bytes);
    /// ```
    fn deploy_program(&mut self, program_id: Pubkey, name: &str, program_bytes: &[u8]);
}

impl ProgramTestExt for AnchorContext {
    fn deploy_program(&mut self, program_id: Pubkey, name: &str, program_bytes: &[u8]) {
        self.svm
            .add_program(program_id, program_bytes)
            .expect("Failed to deploy program");
        self.alias(program_id, name);
    }
}
