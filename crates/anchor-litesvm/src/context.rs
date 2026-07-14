use crate::account::AccountError;
use crate::program::Program;
use anchor_lang::AccountDeserialize;
use anchor_litesvm_compat::{Hash, Keypair, LiteSVM, Signature, Signer, Transaction};
use litesvm_utils::naming::{
    deterministic_keypair, Aliases, ErrorNames, EventRegistry, InstructionNames,
};
use litesvm_utils::{InstructionInfo, TestHelpers, TransactionHelpers, TransactionResult};
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;

/// Production-compatible testing context for Anchor programs.
///
/// Provides the exact same API as anchor-client but works directly with LiteSVM,
/// eliminating RPC overhead while maintaining identical syntax for tests and production.
pub struct AnchorContext {
    /// Direct access to the underlying LiteSVM instance
    pub svm: LiteSVM,
    /// The Anchor program ID
    pub program_id: Pubkey,
    /// Pubkey-to-friendly-name table used by the context-level
    /// [`send_ok`](Self::send_ok) / [`send_err`](Self::send_err) /
    /// [`send_err_named`](Self::send_err_named) helpers and stashed on
    /// returned [`TransactionResult`]s so a chained `print_logs()` reads it
    /// implicitly. Extend via [`alias`](Self::alias).
    pub aliases: Aliases,
    /// The payer keypair
    payer: Keypair,
    /// The program instance for instruction building
    program: Program,
    /// Discriminator-to-name table for programs without an IDL, attached to
    /// every send. For the Anchor path names come from the program's own log
    /// line, so this stays empty; populate it via
    /// [`register_instruction`](Self::register_instruction) /
    /// [`register_program_instructions`](Self::register_program_instructions)
    /// for Pinocchio and other hand-rolled programs.
    instruction_names: InstructionNames,
    /// Custom-error-code-to-name table, attached to every send so a
    /// `ProgramError::Custom(n)` can be matched by name in
    /// [`assert_error`](litesvm_utils::TransactionResult::assert_error) /
    /// [`send_err_named`](Self::send_err_named). The failure-path twin of
    /// `instruction_names`; populated via
    /// [`register_error`](Self::register_error) /
    /// [`register_program_errors`](Self::register_program_errors).
    error_names: ErrorNames,
    /// Event decoders, attached to every send so a `Program data:` payload can
    /// be decoded by name and fields via `result.event_registry()`. Empty
    /// until an event type is registered via
    /// [`register_event`](Self::register_event) /
    /// [`register_events_from_idl`](Self::register_events_from_idl).
    event_registry: EventRegistry,
}

impl AnchorContext {
    /// Create a new AnchorContext with an existing LiteSVM instance
    ///
    /// Note: This creates a default payer and funds it. For more control,
    /// use AnchorLiteSVM builder.
    ///
    /// # Example
    /// ```no_run
    /// use anchor_litesvm::LiteSVM;
    /// use anchor_litesvm::AnchorContext;
    /// use solana_program::pubkey::Pubkey;
    ///
    /// let mut svm = LiteSVM::new();
    /// let program_id = Pubkey::new_unique();
    /// let ctx = AnchorContext::new(svm, program_id);
    /// ```
    pub fn new(mut svm: LiteSVM, program_id: Pubkey) -> Self {
        // Create a default payer and fund it
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

        let program = Program::new(program_id);

        Self {
            svm,
            program_id,
            aliases: Aliases::default(),
            payer,
            program,
            instruction_names: InstructionNames::new(),
            error_names: ErrorNames::new(),
            event_registry: EventRegistry::new(),
        }
    }

    /// Create a new AnchorContext with a specific payer
    pub(crate) fn new_with_payer(svm: LiteSVM, program_id: Pubkey, payer: Keypair) -> Self {
        let program = Program::new(program_id);

        Self {
            svm,
            program_id,
            aliases: Aliases::default(),
            payer,
            program,
            instruction_names: InstructionNames::new(),
            error_names: ErrorNames::new(),
            event_registry: EventRegistry::new(),
        }
    }

    /// Get a copy of the program instance for building instructions.
    ///
    /// Simplified API for testing without RPC overhead:
    ///
    /// # Example
    /// ```ignore
    /// let ix = ctx.program()
    ///     .accounts(my_program::client::accounts::MyInstruction { ... })
    ///     .args(my_program::client::args::MyInstruction { ... })
    ///     .instruction()?;
    /// ```
    pub fn program(&self) -> Program {
        self.program
    }

    /// Get the payer keypair
    pub fn payer(&self) -> &Keypair {
        &self.payer
    }

    /// Derive a PDA for *this* program, the id this context was built with, so
    /// you supply only the seeds. This is the common case, and it can't take
    /// the wrong program id. The generic
    /// [`get_pda`](litesvm_utils::TestHelpers::get_pda) on `ctx.svm` takes an
    /// explicit id and is for the rarer job of deriving *another* program's PDA
    /// (a Metaplex metadata account, say).
    pub fn pda(&self, seeds: &[&[u8]]) -> Pubkey {
        Pubkey::find_program_address(seeds, &self.program_id).0
    }

    /// [`pda`](Self::pda) with the bump, for instructions that take it.
    pub fn pda_with_bump(&self, seeds: &[&[u8]]) -> (Pubkey, u8) {
        Pubkey::find_program_address(seeds, &self.program_id)
    }

    /// Register `pubkey -> label` in the context's alias table. Later
    /// inserts shadow earlier ones, so this also serves as a rename when
    /// an actor's role changes mid-test (e.g. authority rotation).
    /// Feed a `(pubkey, name)` program table into the alias layer: the
    /// consumption end of a `Bundle`'s generated `injected_programs()`
    /// (and any other table of the same shape), the way
    /// `register_program_instructions` consumes the Discriminator tables.
    /// `ctx.alias_programs(&Make::injected_programs())` and every injected
    /// program's alias is registered with zero per-program calls.
    pub fn alias_programs(&mut self, table: &[(Pubkey, &str)]) -> &mut Self {
        for (pubkey, name) in table {
            self.alias(*pubkey, *name);
        }
        self
    }

    pub fn alias(&mut self, pubkey: Pubkey, label: impl Into<String>) -> &mut Self {
        self.aliases.add(pubkey, label);
        self
    }

    /// Register `discriminator -> name` for a program so its instructions can
    /// be matched by name, instead of only the raw discriminator. For programs
    /// without an IDL (Pinocchio and other hand-rolled programs), where there
    /// is no `Program log: Instruction: <Name>` line for the runtime to emit.
    /// `discriminator` is the one-byte tag at `data[0]`, the common Pinocchio
    /// shape; for a multi-byte scheme use [`InstructionNames::register`]
    /// directly. The table rides along on every subsequent send. Chainable.
    ///
    /// ```ignore
    /// ctx.register_instruction(PROGRAM_ID, 0, "Make")
    ///    .register_instruction(PROGRAM_ID, 1, "Take")
    ///    .register_instruction(PROGRAM_ID, 2, "Cancel");
    /// ```
    pub fn register_instruction(
        &mut self,
        program_id: Pubkey,
        discriminator: u8,
        name: impl Into<String>,
    ) -> &mut Self {
        self.instruction_names
            .register_byte(program_id, discriminator, name);
        self
    }

    /// Register a batch of one-byte `(discriminator, name)` pairs for a program
    /// in one call: the shape a `define_instructions!`-style macro emits for the
    /// program's whole instruction set. Equivalent to a
    /// [`register_instruction`](Self::register_instruction) per pair. Chainable.
    ///
    /// ```ignore
    /// ctx.register_program_instructions(
    ///     PROGRAM_ID,
    ///     &[(0, "Make"), (1, "Take"), (2, "Cancel")],
    /// );
    /// ```
    pub fn register_program_instructions(
        &mut self,
        program_id: Pubkey,
        entries: &[(u8, &str)],
    ) -> &mut Self {
        for (disc, name) in entries {
            self.instruction_names
                .register_byte(program_id, *disc, *name);
        }
        self
    }

    /// Register an Anchor event type so its `emit!`ed `Program data:` payloads
    /// can be decoded by name and fields via `result.event_registry()`,
    /// instead of hand-parsing the raw base64 blob the runtime logs.
    /// Chainable; call once per event.
    ///
    /// The event's `Debug` output supplies the field body, and `E` must
    /// therefore implement `Debug`; add `#[derive(Debug)]` to the `#[event]`
    /// if it doesn't already.
    ///
    /// The concrete event type lives only inside the decoder closure built here:
    /// `litesvm-utils` stores a type-erased `Fn(&[u8]) -> Option<String>` and
    /// never names an Anchor type (it carries no `anchor-lang` dependency). The
    /// closure pulls the type from `E::try_from_slice`; the 8-byte
    /// `E::DISCRIMINATOR` is the registry key.
    ///
    /// ```ignore
    /// ctx.register_event::<my_program::events::Transfer>();
    /// ```
    pub fn register_event<E>(&mut self) -> &mut Self
    where
        E: anchor_lang::Discriminator + anchor_lang::AnchorDeserialize + std::fmt::Debug + 'static,
    {
        // `Discriminator::DISCRIMINATOR` is a byte slice (8 bytes for an event);
        // copy the leading bytes into the registry's fixed-size key.
        let mut disc = [0u8; 8];
        let src: &[u8] = E::DISCRIMINATOR;
        let n = src.len().min(8);
        disc[..n].copy_from_slice(&src[..n]);

        // The display name: the last segment of the fully-qualified type name
        // (`my_program::events::Transfer` -> `Transfer`).
        let name = std::any::type_name::<E>()
            .rsplit("::")
            .next()
            .unwrap_or("Event")
            .to_string();

        // Derived `Debug` prints `Transfer { field: val, .. }`; parse that into
        // `(field, value)` pairs (the type name is dropped, it's already `name`).
        self.event_registry.register(
            disc,
            name,
            std::sync::Arc::new(move |bytes: &[u8]| {
                let e = E::try_from_slice(bytes).ok()?;
                Some(crate::event_idl::debug_to_pairs(&format!("{e:?}")))
            }),
        );
        self
    }

    /// Auto-register *every* event in `idl_json` (an Anchor IDL) for decoding,
    /// so `result.event_registry()` can decode any of the program's events by
    /// name and fields with no per-event [`register_event`](Self::register_event)
    /// call. Embed the IDL with `include_str!` so it travels with the test:
    ///
    /// ```ignore
    /// ctx.register_events_from_idl(include_str!("../../target/idl/my_program.json"));
    /// ```
    ///
    /// Fields are formatted from the IDL's type tags (`pubkey`, `u64`, ...)
    /// rather than the event's own `Debug`; an event whose fields the decoder
    /// can't model (a `defined` struct, an `option`, a `vec`) keeps its raw
    /// form rather than risk a mis-aligned read. For full-`Debug` decoding of a
    /// specific event, use [`register_event`](Self::register_event); the two
    /// compose (a later typed registration overrides the IDL one). Panics on
    /// invalid IDL JSON.
    pub fn register_events_from_idl(&mut self, idl_json: &str) -> &mut Self {
        crate::event_idl::register_all(&mut self.event_registry, idl_json);
        self
    }

    /// Register `code -> name` for a program's custom error, so a
    /// `ProgramError::Custom(code)` it returns can be matched by name via
    /// [`send_err_named`](Self::send_err_named) / `assert_error(name)`. The
    /// failure-path twin of [`register_instruction`](Self::register_instruction),
    /// for programs without an IDL (where the runtime logs only the bare
    /// `custom program error: 0x<code>`). The table rides along on every
    /// subsequent send. Chainable.
    ///
    /// ```ignore
    /// ctx.register_error(PROGRAM_ID, 7, "InvalidAmount");
    /// ```
    pub fn register_error(
        &mut self,
        program_id: Pubkey,
        code: u32,
        name: impl Into<String>,
    ) -> &mut Self {
        self.error_names.register(program_id, code, name);
        self
    }

    /// The event-decoder table registered so far. Test-only: the registry is
    /// threaded onto every send automatically, so production code never needs to
    /// reach in here.
    #[cfg(test)]
    pub(crate) fn event_registry(&self) -> &EventRegistry {
        &self.event_registry
    }

    /// Register a batch of `(code, name)` pairs for a program's error set in one
    /// call: the shape a `define_error_set!`-style macro emits. Equivalent to a
    /// [`register_error`](Self::register_error) per pair. Chainable.
    ///
    /// ```ignore
    /// ctx.register_program_errors(PROGRAM_ID, EscrowError::error_names());
    /// ```
    pub fn register_program_errors(
        &mut self,
        program_id: Pubkey,
        entries: &[(u32, &str)],
    ) -> &mut Self {
        for (code, name) in entries {
            self.error_names.register(program_id, *code, *name);
        }
        self
    }

    /// Cast a funded, named signer: a deterministic keypair (reproducible per
    /// program + name), airdropped 100 SOL, and aliased under `name`. The name
    /// rides along as the cast description, so a scenario reads as its dramatis
    /// personae rather than anonymous setup: `let owner = ctx.cast_actor("owner");`.
    pub fn cast_actor(&mut self, name: &str) -> Keypair {
        self.cast_actor_with_sol(name, 100_000_000_000)
    }

    /// [`cast_actor`](Self::cast_actor) with an explicit lamport balance instead
    /// of the 100 SOL float. Reach for it when a scenario asserts on exact SOL:
    /// cast at the precise stake rather than casting at 100 SOL and correcting
    /// after. Same determinism, alias, and cast-name uniqueness; only the
    /// funding amount differs.
    pub fn cast_actor_with_sol(&mut self, name: &str, lamports: u64) -> Keypair {
        self.track_cast(name);
        let kp = deterministic_keypair(&self.program_id.to_string(), name);
        self.svm
            .airdrop(&kp.pubkey(), lamports)
            .expect("airdrop to a freshly-cast actor");
        self.alias(kp.pubkey(), name);
        kp
    }

    /// Record `name` as a cast on this context, asserting it is the first use.
    /// Casts seed keypairs and register aliases from their name, so a repeat
    /// would silently fork one name across two identities; this is the
    /// duplicate-label guard the `cast_*` vocabulary shares.
    fn track_cast(&mut self, name: &str) {
        assert!(
            self.aliases.register_cast(name),
            "cast name {name:?} already used in this scenario; cast names seed \
             keypairs and register aliases, so a duplicate would alias two casts \
             to one identity. Give this cast a distinct name."
        );
    }

    /// Cast a named passive account: a deterministic, rent-funded pubkey aliased
    /// under `name`. For a recipient / target that isn't a signer.
    pub fn cast_account(&mut self, name: &str) -> Pubkey {
        self.track_cast(name);
        let pk = deterministic_keypair(&self.program_id.to_string(), name).pubkey();
        self.svm
            .airdrop(&pk, 1_000_000_000)
            .expect("rent-fund a freshly-cast account");
        self.alias(pk, name);
        pk
    }

    /// Cast a token mint: a deterministic mint account (reproducible per
    /// program + name, the same derivation [`cast_actor`](Self::cast_actor)
    /// uses), created and initialized under `authority` with `decimals`, then
    /// aliased under `name`. Returns the mint address. The authority pays the
    /// mint's rent and signs its creation, so cast it first:
    ///
    /// ```ignore
    /// let issuer = ctx.cast_actor("issuer");
    /// let usdc = ctx.cast_mint("USDC", &issuer, 6); // aliased "USDC"
    /// ```
    ///
    /// This completes the cast vocabulary on the token side: where a suite
    /// would otherwise derive a mint keypair, call `create_token_mint_at`, and
    /// `alias` it as three separate steps, the mint names itself as it is cast.
    pub fn cast_mint(&mut self, name: &str, authority: &Keypair, decimals: u8) -> Pubkey {
        self.track_cast(name);
        let mint_kp = deterministic_keypair(&self.program_id.to_string(), name);
        self.svm
            .create_token_mint_at(authority, &mint_kp, decimals)
            .expect("create a freshly-cast mint");
        let mint = mint_kp.pubkey();
        self.alias(mint, name);
        mint
    }

    /// Fund `owner`'s associated token account for `mint`: create the ATA,
    /// alias it under the composed `"<owner>/<mint>"` name, and mint `amount`
    /// from `authority` (skipped when `amount` is 0, leaving a real but empty
    /// account a later transfer can land in). Returns the ATA address. `owner`
    /// pays the ATA rent and signs; `authority` is the mint's authority.
    ///
    /// This is the holder side of the cast vocabulary: cast the owner and the
    /// mint, then hand the owner a balance in one call, instead of the
    /// create-ATA / mint-to / alias-ATA trio every funded-holder setup repeats.
    /// Alias the owner and mint first (e.g. with `cast_actor` / `cast_mint`) so
    /// the composed name reads "Alice/USDC" rather than two short hex stubs.
    ///
    /// ```ignore
    /// let issuer = ctx.cast_actor("issuer");
    /// let alice = ctx.cast_actor("Alice");
    /// let usdc = ctx.cast_mint("USDC", &issuer, 6);
    /// let alice_usdc = ctx.fund_ata(&alice, &usdc, &issuer, 1_000_000); // aliased "Alice/USDC"
    /// ```
    pub fn fund_ata(
        &mut self,
        owner: &Keypair,
        mint: &Pubkey,
        authority: &Keypair,
        amount: u64,
    ) -> Pubkey {
        let ata = self
            .svm
            .create_associated_token_account(mint, owner)
            .expect("create an ATA for a funded holder");
        self.alias_ata(&owner.pubkey(), mint);
        if amount > 0 {
            self.svm
                .mint_to(mint, &ata, authority, amount)
                .expect("mint to a funded holder");
        }
        ata
    }

    /// Resolve `pubkey` to its registered alias, or a short `<8>…<4>` form
    /// when it isn't aliased. Shorthand for `self.aliases.label(&pubkey)`.
    pub fn label(&self, pubkey: &Pubkey) -> String {
        self.aliases.label(pubkey)
    }

    /// Derive the associated token account for `(owner, mint)`, register it
    /// under the composed name `"<owner>/<mint>"` drawn from the alias table,
    /// and return its address. Name the leaves first (the owner and the mint),
    /// then compose every token-account name off them in one line:
    ///
    /// ```ignore
    /// ctx.alias(alice.pubkey(), "Alice");
    /// ctx.alias(mint_x, "X");
    /// let ata = ctx.alias_ata(&alice.pubkey(), &mint_x); // aliased "Alice/X"
    /// ```
    ///
    /// Use [`alias_ata_as`](Self::alias_ata_as) when a conventional name reads
    /// better than the composed one.
    pub fn alias_ata(&mut self, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        let label = format!("{}/{}", self.label(owner), self.label(mint));
        self.alias_ata_as(owner, mint, label)
    }

    /// [`alias_ata`](Self::alias_ata) with a caller-chosen label instead of the
    /// canonical `<owner>/<mint>`. The escape hatch for an account that reads
    /// better under a conventional name (a pool's `"VaultX"` rather than
    /// `"Pool/X"`); the derivation is identical, only the label differs.
    pub fn alias_ata_as(
        &mut self,
        owner: &Pubkey,
        mint: &Pubkey,
        label: impl Into<String>,
    ) -> Pubkey {
        let ata = spl_associated_token_account::get_associated_token_address(owner, mint);
        self.alias(ata, label);
        ata
    }

    /// Start a fluent [`Tx`](crate::tx::Tx) chain: build + send +
    /// expect in one statement, with the success and negative paths
    /// sharing every step up to the terminator. Replaces the per-verb
    /// `_ok`/`_expecting` pair that hand-rolled helpers tend to grow.
    ///
    /// ```ignore
    /// ctx.tx(&[&signer])
    ///    .build(SwapBundle::from((&pool, &user)), instruction::Swap { kind, dir })
    ///    .send_ok()
    ///    .print_logs();
    /// ```
    pub fn tx<'a>(&'a mut self, signers: &'a [&'a Keypair]) -> crate::tx::Tx<'a> {
        crate::tx::Tx::new(self, signers)
    }

    /// Send an ix expected to succeed, with the printed-log aliases drawn
    /// from `self.aliases`. Returned [`TransactionResult`] carries the
    /// aliases internally, so `.print_logs()` works with no argument. Thin
    /// wrapper over [`TransactionHelpers::send_ok`] that removes the
    /// per-call `&Aliases` thread.
    pub fn send_ok(&mut self, ix: Instruction, signers: &[&Keypair]) -> TransactionResult {
        let result = self.svm.send_ok(ix, signers, &self.aliases);
        self.attach_context_tables(result)
    }

    /// Send an ix expected to fail (any error). Aliases drawn from
    /// `self.aliases`. Companion to [`send_ok`](Self::send_ok).
    pub fn send_err(&mut self, ix: Instruction, signers: &[&Keypair]) -> TransactionResult {
        let result = self.svm.send_err(ix, signers, &self.aliases);
        self.attach_context_tables(result)
    }

    /// Send an ix expected to fail with `error_name`. Matched against the logs,
    /// the runtime error field, *and* the registered error-name table, so a
    /// Pinocchio `ProgramError::Custom(7)` matches `"InvalidAmount"` once its
    /// code is registered (see
    /// [`register_error`](Self::register_error)). Aliases drawn from
    /// `self.aliases`. Companion to [`send_ok`](Self::send_ok).
    ///
    /// Unlike the raw [`TransactionHelpers::send_err_named`], this enriches the
    /// result (alias + error-name table) *before* matching, which is what lets
    /// the name match see the registry. The raw helper asserts on the bare
    /// result, so it can only match names that appear verbatim in the logs.
    pub fn send_err_named(
        &mut self,
        ix: Instruction,
        signers: &[&Keypair],
        error_name: &str,
    ) -> TransactionResult {
        let result = self.svm.send_err(ix, signers, &self.aliases);
        self.attach_context_tables(result).assert_error(error_name)
    }

    /// Send a full instruction list as one transaction. Unlike
    /// [`send_ok`](Self::send_ok) / [`send_err`](Self::send_err) it asserts
    /// nothing (the caller decides, since a multi-ix dispatch may be expected
    /// to pass or fail); it returns the [`TransactionResult`] so the caller can
    /// `assert_success` / `assert_error`.
    pub fn send_instructions(
        &mut self,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> TransactionResult {
        let result = self
            .svm
            .send_instructions(ixs, signers)
            .expect("send_instructions: build a valid transaction")
            .with_aliases(self.aliases.clone());
        self.attach_context_tables(result)
    }

    /// Attach this context's name and event tables to a freshly built
    /// [`TransactionResult`], so `assert_error` / `send_err_named` can resolve
    /// a registered name, and `result.event_registry()` can decode this
    /// context's registered events. Shared tail of every context-level send.
    fn attach_context_tables(&self, result: TransactionResult) -> TransactionResult {
        result
            .with_instruction_names(self.instruction_names.clone())
            .with_error_names(self.error_names.clone())
            .with_event_registry(self.event_registry.clone())
    }

    /// Execute a single instruction using LiteSVM
    ///
    /// This is a convenience method for executing instructions.
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

        // Capture the ix info for the printed-logs header before the
        // transaction below borrows `instruction`. `from_instruction`
        // clones only the data bytes, which is what we need anyway.
        let info = InstructionInfo::from_instruction(&instruction);
        // Build and sign the transaction
        // Fresh by default: see TransactionHelpers::send_instruction.
        self.svm.expire_blockhash();
        let tx = Transaction::new_signed_with_payer(
            std::slice::from_ref(&instruction),
            Some(&payer_pubkey),
            signers,
            self.svm.latest_blockhash(),
        );
        let message = tx.message.clone();

        // Execute the transaction, then attach this context's aliases and name
        // tables so its views name accounts and resolve failure names exactly
        // as a `send_ok` result does.
        match self.svm.send_transaction(tx) {
            Ok(result) => {
                let r = TransactionResult::new(result, Some(info), message)
                    .with_aliases(self.aliases.clone());
                Ok(self.attach_context_tables(r))
            }
            Err(failed) => {
                let r = TransactionResult::new_failed(
                    format!("{:?}", failed.err),
                    failed.meta,
                    Some(info),
                    message,
                )
                .with_aliases(self.aliases.clone());
                Ok(self.attach_context_tables(r))
            }
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
        // Fresh by default: see TransactionHelpers::send_instruction.
        self.svm.expire_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer_pubkey),
            signers,
            self.svm.latest_blockhash(),
        );
        let message = tx.message.clone();

        // Execute the transaction, then attach this context's aliases and name
        // tables so its views match a `send_ok` result.
        match self.svm.send_transaction(tx) {
            Ok(result) => {
                let r = TransactionResult::new(result, None, message)
                    .with_aliases(self.aliases.clone());
                Ok(self.attach_context_tables(r))
            }
            Err(failed) => {
                let r = TransactionResult::new_failed(
                    format!("{:?}", failed.err),
                    failed.meta,
                    None,
                    message,
                )
                .with_aliases(self.aliases.clone());
                Ok(self.attach_context_tables(r))
            }
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
    /// # use anchor_litesvm::LiteSVM;
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
    /// let account: MyAccount = ctx.try_load(&account_pubkey).unwrap();
    /// ```
    pub fn try_load<T>(&self, address: &Pubkey) -> Result<T, AccountError>
    where
        T: AccountDeserialize,
    {
        let account_data = self
            .svm
            .get_account(address)
            .ok_or(AccountError::AccountNotFound(*address))?;

        // Deserialize the account data
        let mut data = account_data.data.as_slice();
        T::try_deserialize(&mut data).map_err(|e| AccountError::DeserializationError(e.to_string()))
    }

    /// Get an Anchor account without discriminator check
    ///
    /// Use this for accounts that don't have the standard Anchor discriminator.
    ///
    /// Note: `try_deserialize_unchecked` handles the discriminator internally,
    /// so we pass the full account data.
    pub fn try_load_unchecked<T>(&self, address: &Pubkey) -> Result<T, AccountError>
    where
        T: AccountDeserialize,
    {
        let account_data = self
            .svm
            .get_account(address)
            .ok_or(AccountError::AccountNotFound(*address))?;

        // Deserialize without discriminator check
        // Note: try_deserialize_unchecked handles the discriminator internally
        let mut data = account_data.data.as_slice();
        T::try_deserialize_unchecked(&mut data)
            .map_err(|e| AccountError::DeserializationError(e.to_string()))
    }

    /// Load an Anchor account, panicking on failure.
    ///
    /// Test-oriented sibling of [`try_load`](Self::try_load): the same fetch
    /// and deserialization, but failures (missing account, wrong discriminator,
    /// deser error) panic with the address and underlying [`AccountError`] in the
    /// message instead of returning a `Result`. Use in tests where a missing or
    /// malformed account is itself a test failure.
    ///
    /// # Example
    /// ```ignore
    /// let escrow: Escrow = ctx.load(&accs.escrow);
    /// assert_eq!(escrow.expiry_utc, Some(expiry));
    /// ```
    pub fn load<T>(&self, address: &Pubkey) -> T
    where
        T: AccountDeserialize,
    {
        self.try_load(address)
            .unwrap_or_else(|e| panic!("failed to load account at {address}: {e}"))
    }

    /// Load an Anchor account without discriminator check, panicking on failure.
    ///
    /// Test-oriented sibling of [`try_load_unchecked`](Self::try_load_unchecked).
    /// Same panic semantics as [`load`](Self::load).
    pub fn load_unchecked<T>(&self, address: &Pubkey) -> T
    where
        T: AccountDeserialize,
    {
        self.try_load_unchecked(address)
            .unwrap_or_else(|e| panic!("failed to load account at {address}: {e}"))
    }

    /// Create a funded account (convenience method)
    pub fn create_funded_account(
        &mut self,
        lamports: u64,
    ) -> Result<Keypair, Box<dyn std::error::Error>> {
        let account = Keypair::new();
        self.svm
            .airdrop(&account.pubkey(), lamports)
            .map_err(|e| format!("Airdrop failed: {:?}", e))?;
        Ok(account)
    }

    /// Airdrop lamports to an account (convenience method)
    pub fn airdrop(
        &mut self,
        pubkey: &Pubkey,
        lamports: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.svm
            .airdrop(pubkey, lamports)
            .map_err(|e| format!("Airdrop failed: {:?}", e))?;
        Ok(())
    }

    /// Get the latest blockhash
    pub fn latest_blockhash(&self) -> Hash {
        self.svm.latest_blockhash()
    }

    /// Check if an account exists
    pub fn account_exists(&self, pubkey: &Pubkey) -> bool {
        self.svm.get_account(pubkey).is_some()
    }
}
