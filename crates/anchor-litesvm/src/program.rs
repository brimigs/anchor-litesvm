//! Simplified instruction builder for LiteSVM testing without RPC overhead.
//!
//! This module provides a clean, testing-focused API that removes unnecessary
//! RPC-layer abstractions like `.request()` and `.remove(0)`.

use crate::buildable::BuildableIx;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{instruction::Instruction, pubkey::Pubkey};

/// A lightweight Program wrapper for building instructions in tests.
///
/// Simplified API for testing without RPC layer abstractions:
/// ```ignore
/// let ix = ctx.program()
///     .accounts(my_program::accounts::Transfer { ... })
///     .args(my_program::instruction::Transfer { ... })
///     .instruction()?;
/// ```
#[derive(Copy, Clone)]
pub struct Program {
    program_id: Pubkey,
}

impl Program {
    /// Create a new Program instance for the given program ID
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }

    /// Start building an instruction with accounts.
    ///
    /// This returns an `InstructionBuilder` that you can chain with `.args()` and `.instruction()`.
    ///
    /// # Example
    /// ```ignore
    /// let ix = ctx.program()
    ///     .accounts(my_program::accounts::Initialize {
    ///         user: user.pubkey(),
    ///         account: data_account,
    ///         system_program: system_program::id(),
    ///     })
    ///     .args(my_program::instruction::Initialize { value: 42 })
    ///     .instruction()?;
    /// ```
    pub fn accounts<T: ToAccountMetas>(self, accounts: T) -> InstructionBuilder {
        InstructionBuilder {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: Vec::new(),
        }
    }

    /// Get the program ID
    pub fn id(&self) -> Pubkey {
        self.program_id
    }

    /// Build an instruction in one call, deriving the accounts struct from a
    /// caller-supplied bundle of pubkeys via [`BuildableIx`].
    ///
    /// Equivalent to:
    ///
    /// ```ignore
    /// self.accounts(<A::Accounts>::from(bundle))
    ///     .args(args)
    ///     .instruction()
    ///     .unwrap()
    /// ```
    ///
    /// but with the args/accounts pairing checked at compile time and no
    /// `Result` to unwrap (an `InstructionData` impl always produces a
    /// non-empty payload).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ix = ctx.program().build_ix(
    ///     DepositBundle { user, state, vault },
    ///     instruction::Deposit { amount: 1_000_000 },
    /// );
    /// ```
    ///
    /// The bundle type `B` is inferred from the `bundle` argument; you only
    /// need to annotate if the args type implements `BuildableIx` for more
    /// than one bundle.
    ///
    /// For negative-path tests that need to override one of the
    /// bundle-derived accounts, see [`Program::build_ix_with`].
    ///
    /// You can always drop back to [`Program::accounts`] +
    /// [`InstructionBuilder::args`] + [`InstructionBuilder::instruction`] if
    /// you need full manual control over the accounts struct.
    pub fn build_ix<B, A>(self, bundle: B, args: A) -> Instruction
    where
        A: BuildableIx<B>,
        B: Into<A::Accounts>,
    {
        let accounts: A::Accounts = bundle.into();
        Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: args.data(),
        }
    }

    /// Build an instruction with a closure that can mutate the bundle-derived
    /// accounts struct before account metas are computed.
    ///
    /// Useful for negative-path tests where you want to deliberately pass a
    /// wrong account to check that the program rejects it. The closure
    /// receives `&mut A::Accounts`, which is a concrete typed struct, so
    /// every field is first-class to rust-analyzer.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Pass the wrong vault_state PDA to verify the program rejects it.
    /// let ix = ctx.program().build_ix_with(
    ///     DepositBundle { user, state, vault },
    ///     instruction::Deposit { amount: 1_000_000 },
    ///     |a| a.vault_state = wrong_pda,
    /// );
    /// ```
    ///
    /// The closure can mutate as many fields as needed; the API surface stays
    /// at exactly two methods regardless of how many overrides a given test
    /// applies.
    pub fn build_ix_with<B, A, F>(self, bundle: B, args: A, modify: F) -> Instruction
    where
        A: BuildableIx<B>,
        B: Into<A::Accounts>,
        F: FnOnce(&mut A::Accounts),
    {
        let mut accounts: A::Accounts = bundle.into();
        modify(&mut accounts);
        Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: args.data(),
        }
    }
}

/// Builder for constructing instructions in a fluent, chainable manner.
///
/// You typically don't create this directly - use `program().accounts()` instead.
pub struct InstructionBuilder {
    program_id: Pubkey,
    accounts: Vec<solana_program::instruction::AccountMeta>,
    data: Vec<u8>,
}

impl InstructionBuilder {
    /// Set the instruction arguments
    ///
    /// # Example
    /// ```ignore
    /// .args(my_program::instruction::Transfer { amount: 1000 })
    /// ```
    pub fn args<T: InstructionData>(mut self, args: T) -> Self {
        self.data = args.data();
        self
    }

    /// Build and return the instruction.
    ///
    /// This is the final method in the chain that produces the `Instruction`.
    ///
    /// # Example
    /// ```ignore
    /// let ix = ctx.program()
    ///     .accounts(...)
    ///     .args(...)
    ///     .instruction()?;
    /// ```
    pub fn instruction(self) -> Result<Instruction, Box<dyn std::error::Error>> {
        if self.data.is_empty() {
            return Err("No instruction data provided. Call .args() before .instruction()".into());
        }

        Ok(Instruction {
            program_id: self.program_id,
            accounts: self.accounts,
            data: self.data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Program;
    use crate::buildable::BuildableIx;
    use anchor_lang::{prelude::*, InstructionData, ToAccountMetas};
    use solana_program::instruction::AccountMeta;
    use solana_program::pubkey::Pubkey;

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
    fn test_simplified_syntax() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let account = Pubkey::new_unique();

        // New simplified syntax for testing
        let program = Program::new(program_id);
        let ix = program
            .accounts(TestAccounts { user, account })
            .args(TestArgs { amount: 100 })
            .instruction()
            .unwrap();

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.data.len() > 8);
    }

    #[derive(Copy, Clone)]
    struct TestBundle {
        user: Pubkey,
        account: Pubkey,
    }

    impl From<TestBundle> for TestAccounts {
        fn from(b: TestBundle) -> Self {
            Self {
                user: b.user,
                account: b.account,
            }
        }
    }

    impl BuildableIx<TestBundle> for TestArgs {
        type Accounts = TestAccounts;
    }

    #[test]
    fn build_ix_constructs_from_bundle() {
        let program_id = Pubkey::new_unique();
        let bundle = TestBundle {
            user: Pubkey::new_unique(),
            account: Pubkey::new_unique(),
        };

        let ix = Program::new(program_id).build_ix(bundle, TestArgs { amount: 42 });

        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts.len(), 2);
        assert_eq!(ix.accounts[0].pubkey, bundle.user);
        assert_eq!(ix.accounts[1].pubkey, bundle.account);
        assert!(ix.data.len() > 8);
    }

    #[test]
    fn build_ix_with_applies_closure_override() {
        let program_id = Pubkey::new_unique();
        let bundle = TestBundle {
            user: Pubkey::new_unique(),
            account: Pubkey::new_unique(),
        };
        let wrong_user = Pubkey::new_unique();

        let ix = Program::new(program_id)
            .build_ix_with(bundle, TestArgs { amount: 42 }, |a| a.user = wrong_user);

        // First account meta reflects the closure-overridden user.
        assert_eq!(ix.accounts[0].pubkey, wrong_user);
        // Second account meta is still the bundle-derived value.
        assert_eq!(ix.accounts[1].pubkey, bundle.account);
    }
}
