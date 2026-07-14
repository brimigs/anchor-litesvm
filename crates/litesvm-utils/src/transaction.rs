//! Transaction execution and result handling utilities
//!
//! This module provides convenient wrappers for executing transactions
//! and handling their results in tests.

use crate::naming::{Aliases, ErrorNames, EventRegistry, InstructionNames};
use anchor_litesvm_compat::{Keypair, LiteSVM, Message, Signer, Transaction, TransactionMetadata};
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
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
/// ```no_run
/// # use litesvm_utils::TransactionHelpers;
/// # use litesvm_utils::LiteSVM;
/// # use solana_program::instruction::Instruction;
/// # use litesvm_utils::Keypair;
/// # let mut svm = LiteSVM::new();
/// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
/// # let signer = Keypair::new();
/// let result = svm.send_instruction(ix, &[&signer]).unwrap()
///     .tap(|r| {
///         assert!(r.has_log("Transfer complete"));
///         println!("Used {} compute units", r.compute_units());
///     })
///     .assert_success();
/// ```
/// The program ID + serialized data of a single top-level instruction.
///
/// Carried on [`TransactionResult`] for single-instruction sends so
/// [`assert_error`](TransactionResult::assert_error) can resolve a custom
/// error code through the registered [`ErrorNames`] table even when the
/// program's own logs only spell out the bare `0x<code>`.
///
/// `None` on `TransactionResult` for batches (multi-ix sends) and for the
/// raw `send_transaction_result` path, since neither carries a single
/// canonical "the instruction" to attribute a failing code to.
#[derive(Debug, Clone)]
pub struct InstructionInfo {
    pub program_id: Pubkey,
    /// Full instruction data. Only the first 1-8 bytes are ever read (by a
    /// discriminator lookup); the rest is preserved because copying a few
    /// hundred bytes per test is cheaper than worrying about which slice
    /// width covers every future decoder.
    pub data: Box<[u8]>,
}

impl InstructionInfo {
    /// Capture program ID + a clone of the data bytes from an
    /// `Instruction`. Used by `send_instruction` / `execute_instruction`
    /// to stash the ix info before the original `Instruction` is moved
    /// into a `Transaction`.
    pub fn from_instruction(ix: &Instruction) -> Self {
        Self {
            program_id: ix.program_id,
            data: ix.data.clone().into_boxed_slice(),
        }
    }
}

pub struct TransactionResult {
    inner: TransactionMetadata,
    /// Top-level instruction info when the caller built the
    /// `TransactionResult` from a single instruction. See [`InstructionInfo`]
    /// for the `None` cases.
    instruction: Option<InstructionInfo>,
    error: Option<String>,
    /// The sent transaction's `Message` (account keys, header, signer count):
    /// the raw material for any signer/authority annotation a caller wants to
    /// derive from who signed what. Read back via [`message`](Self::message).
    message: Message,
    /// Pubkey-to-friendly-name table used by
    /// [`print_logs`](Self::print_logs). Set via
    /// [`with_aliases`](Self::with_aliases); `None` falls back to
    /// [`Aliases::default`] (well-known programs only).
    aliases: Option<Aliases>,
    /// Discriminator-to-name table for programs without an IDL (Pinocchio and
    /// other hand-rolled programs). Set via
    /// [`with_instruction_names`](Self::with_instruction_names) and read back
    /// via [`instruction_names`](Self::instruction_names); empty by default.
    instruction_names: InstructionNames,
    /// Custom-error-code-to-name table, the failure-path twin of
    /// `instruction_names`: lets a Pinocchio `ProgramError::Custom(n)` render
    /// and match by name (`InvalidAmount`) instead of `0x<n>`. Empty by
    /// default (Anchor failures carry their own name in the logs). Set via
    /// [`with_error_names`](Self::with_error_names); consulted inside
    /// [`assert_error`](Self::assert_error) when the top-level instruction's
    /// program failed with a custom code. See [`ErrorNames`].
    error_names: ErrorNames,
    /// Decoders for registered Anchor events, so a `Program data:` payload
    /// can be decoded by name and fields instead of staying raw base64.
    /// Empty by default. Set via
    /// [`with_event_registry`](Self::with_event_registry) and read back via
    /// [`event_registry`](Self::event_registry). See [`EventRegistry`].
    event_registry: EventRegistry,
}

/// One log line, display-ready: a `Program data:` payload the attached
/// event registry can decode renders as its badge, fields alias-resolved;
/// everything else, including a payload no decoder matches, keeps its raw
/// text with registered pubkeys substituted. The raw line surviving an
/// unmatched decode is deliberate: rendering must never drop information.
fn render_log_line(log: &str, events: &EventRegistry, aliases: &Aliases) -> String {
    if let Some(payload) = log.strip_prefix("Program data: ") {
        if let Some(info) = events.decode_logged(payload) {
            return info.badge_resolved(aliases);
        }
    }
    aliases.substitute_in_text(log)
}

/// The invariant working set of one [`TransactionResult::tree_string`]
/// walk: naming tables, the legend accumulated in first-use order, the
/// transaction signers, the failure leaf, and the event decoders. Frames
/// and cursor state travel as arguments; everything here is constant
/// across the recursion.
struct TreeRender<'a> {
    aliases: &'a Aliases,
    /// Names the default table already knows (well-known programs); those
    /// never earn a legend row.
    well_known: Aliases,
    legend: Vec<(String, Pubkey)>,
    signer_names: Vec<String>,
    error_leaf: Option<String>,
    events: &'a EventRegistry,
}

impl TreeRender<'_> {
    /// A pubkey's display name; a test-registered name is recorded for the
    /// legend the first time it appears.
    fn label(&mut self, pk: &Pubkey) -> String {
        let name = self.aliases.label(pk);
        if self.well_known.resolve_by_pubkey(pk).is_none()
            && self.aliases.resolve_by_pubkey(pk).is_some()
            && !self.legend.iter().any(|(n, _)| n == &name)
        {
            self.legend.push((name.clone(), *pk));
        }
        name
    }

    fn write_frame(
        &mut self,
        out: &mut String,
        frame: &anchor_litesvm_compat::CpiFrame,
        prefix: &str,
        is_last: bool,
        depth: usize,
    ) {
        use anchor_litesvm_compat::{CpiOutcome, FrameLog};
        use std::fmt::Write as _;

        let connector = if is_last {
            "\u{2514}\u{2500}\u{2500} "
        } else {
            "\u{251c}\u{2500}\u{2500} "
        };
        let mark = match &frame.outcome {
            CpiOutcome::Success => "\u{2713}",
            CpiOutcome::Failed { .. } => "\u{2717}",
            CpiOutcome::Truncated => "\u{22ef}",
        };
        let cu = match frame.compute_units {
            Some(c) => format!("{}cu", c.consumed),
            None => "(no cu)".to_string(),
        };
        let mut line = self.label(&frame.program_id);
        if let Some(ix) = &frame.instruction_name {
            line = format!("{line}::{ix}");
        }
        write!(out, "{prefix}{connector}{line} [{depth}] {mark} {cu}").unwrap();
        if depth == 1 && !self.signer_names.is_empty() {
            write!(out, "  signer={}", self.signer_names.join(",")).unwrap();
        }
        writeln!(out).unwrap();

        let child_prefix = if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}\u{2502}   ")
        };
        // Decoded events render inside their frame; undecodable data and
        // plain msg lines stay out of the tree (the flat view keeps them).
        let badges: Vec<String> = frame
            .logs
            .iter()
            .filter_map(|l| match l {
                FrameLog::Data(payload) => self
                    .events
                    .decode_logged(payload)
                    .map(|info| info.badge_resolved(self.aliases)),
                FrameLog::Msg(_) => None,
            })
            .collect();
        // The resolved failure renders once, as a leaf on the top-level
        // failed frame; inner failed frames keep their ✗ mark without
        // restating the same error down the spine.
        let failed = depth == 1 && matches!(frame.outcome, CpiOutcome::Failed { .. });
        let tail_count = badges.len() + usize::from(failed);
        let n_children = frame.children.len();
        for (i, child) in frame.children.iter().enumerate() {
            let last = i == n_children - 1 && tail_count == 0;
            self.write_frame(out, child, &child_prefix, last, depth + 1);
        }
        for (i, badge) in badges.iter().enumerate() {
            let last = i == badges.len() - 1 && !failed;
            let c = if last {
                "\u{2514}\u{2500}\u{2500} "
            } else {
                "\u{251c}\u{2500}\u{2500} "
            };
            writeln!(out, "{child_prefix}{c}{badge}").unwrap();
        }
        if failed {
            let msg = match &frame.outcome {
                CpiOutcome::Failed { message } => self
                    .error_leaf
                    .as_deref()
                    .map(str::to_string)
                    .or_else(|| message.clone())
                    .unwrap_or_else(|| "failed".to_string()),
                _ => unreachable!(),
            };
            writeln!(out, "{child_prefix}\u{2514}\u{2500}\u{2500} Error: {msg}").unwrap();
        }
    }
}

impl TransactionResult {
    /// Create a new TransactionResult wrapper for a successful transaction.
    ///
    /// Pass `Some(_)` for `instruction` only when the transaction wraps a
    /// single instruction (so a later failure can be attributed to it);
    /// pass `None` for batches.
    pub fn new(
        result: TransactionMetadata,
        instruction: Option<InstructionInfo>,
        message: Message,
    ) -> Self {
        Self {
            inner: result,
            instruction,
            error: None,
            message,
            aliases: None,
            instruction_names: InstructionNames::new(),
            error_names: ErrorNames::new(),
            event_registry: EventRegistry::new(),
        }
    }

    /// Create a new TransactionResult wrapper for a failed transaction.
    ///
    /// See [`new`](Self::new) for the `instruction` convention.
    pub fn new_failed(
        error: String,
        result: TransactionMetadata,
        instruction: Option<InstructionInfo>,
        message: Message,
    ) -> Self {
        Self {
            inner: result,
            instruction,
            error: Some(error),
            message,
            aliases: None,
            instruction_names: InstructionNames::new(),
            error_names: ErrorNames::new(),
            event_registry: EventRegistry::new(),
        }
    }

    /// Attach an alias table that drives subsequent
    /// [`print_logs`](Self::print_logs) calls. Returns `self` for chaining;
    /// the table is cloned in (cheap), so the caller keeps ownership of the
    /// original.
    pub fn with_aliases(mut self, aliases: Aliases) -> Self {
        self.aliases = Some(aliases);
        self
    }

    /// Attach a discriminator-to-name table so instructions from a program
    /// without an IDL can be named instead of falling back to the bare
    /// program alias. Cloned in (cheap); chainable. See [`InstructionNames`].
    pub fn with_instruction_names(mut self, names: InstructionNames) -> Self {
        self.instruction_names = names;
        self
    }

    /// The instruction-name table attached via
    /// [`with_instruction_names`](Self::with_instruction_names).
    pub fn instruction_names(&self) -> &InstructionNames {
        &self.instruction_names
    }

    /// Attach a custom-error-code-to-name table so a `ProgramError::Custom`
    /// from a program without an IDL renders and matches by name. Cloned in
    /// (cheap); chainable. See [`ErrorNames`].
    pub fn with_error_names(mut self, errors: ErrorNames) -> Self {
        self.error_names = errors;
        self
    }

    /// The error-name table attached via
    /// [`with_error_names`](Self::with_error_names).
    pub fn error_names(&self) -> &ErrorNames {
        &self.error_names
    }

    /// Attach a table of event decoders so a `Program data:` payload can be
    /// decoded by name and fields instead of staying raw base64. Cloned in
    /// (cheap: the registry's decoders are `Arc`d); chainable. See
    /// [`EventRegistry`].
    pub fn with_event_registry(mut self, events: EventRegistry) -> Self {
        self.event_registry = events;
        self
    }

    /// The event registry attached via
    /// [`with_event_registry`](Self::with_event_registry).
    pub fn event_registry(&self) -> &EventRegistry {
        &self.event_registry
    }

    /// Assert that the transaction succeeded, panic with logs if it failed.
    ///
    /// Consumes and returns `self` so the result can flow into a further
    /// chain (`...assert_success().tap(...)`) or be bound at chain end
    /// (`let result = svm.send_ok(...).assert_success();`). Read-only
    /// inspection inside a chain goes through [`tap`](Self::tap), which
    /// borrows for the closure and returns the owned value back.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::TransactionHelpers;
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let payer = Keypair::new();
    /// let result = svm.send_instruction(ix, &[&payer]).unwrap().assert_success();
    /// ```
    pub fn assert_success(self) -> Self {
        assert!(
            self.error.is_none(),
            "Transaction failed: {}\nLogs:\n{}",
            self.error.as_ref().unwrap_or(&"Unknown error".to_string()),
            self.logs().join("\n")
        );
        self
    }

    /// Assert that the transaction succeeded AND a caller-supplied
    /// predicate holds on the result. Panics with context on either
    /// failure. Consumes and returns `self`.
    ///
    /// Useful for baking an additional check (compute units, log
    /// presence, custom invariant) into a single chain step rather than
    /// reaching for [`tap`](Self::tap) + a separate
    /// [`assert_success`](Self::assert_success).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::{Aliases, TransactionHelpers};
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let payer = Keypair::new();
    /// # let aliases = Aliases::default();
    /// let r = svm.send_ok(ix, &[&payer], &aliases)
    ///     .assert_success_with(|r| r.compute_units() < 100_000);
    /// ```
    pub fn assert_success_with<F>(self, predicate: F) -> Self
    where
        F: FnOnce(&Self) -> bool,
    {
        assert!(
            self.error.is_none(),
            "Transaction failed: {}\nLogs:\n{}",
            self.error.as_ref().unwrap_or(&"Unknown error".to_string()),
            self.logs().join("\n")
        );
        assert!(
            predicate(&self),
            "Predicate failed on successful transaction.\nLogs:\n{}",
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

    /// The fee (in lamports) the SVM reported for this transaction. Note
    /// that fees are *not* refunded on failure, so a failed tx still
    /// reports the lamports the fee payer was charged. The value is just
    /// the underlying `TransactionMetadata.fee` lifted to a method for
    /// consistency with [`compute_units`](Self::compute_units).
    pub fn fee(&self) -> u64 {
        self.inner.fee
    }

    /// The custom program error code the runtime logged, if any.
    ///
    /// Read from the raw logs, not the `error` field: `error` is
    /// `TransactionError`'s `Debug` form (whose shape isn't guaranteed),
    /// whereas `custom program error: 0x<code>` is the line the runtime
    /// itself emits on a `ProgramError::Custom` failure. Returns the first
    /// such code in log order (the failing frame's).
    pub fn error_code(&self) -> Option<u32> {
        const NEEDLE: &str = "custom program error: 0x";
        self.inner.logs.iter().find_map(|line| {
            let rest = line.split(NEEDLE).nth(1)?;
            let hex: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
            u32::from_str_radix(&hex, 16).ok()
        })
    }

    /// The failing custom error's registered name, if this result carries a
    /// single top-level instruction, the runtime logged a custom error code,
    /// and that code is registered in [`error_names`](Self::error_names) for
    /// the instruction's program.
    fn resolved_error_name(&self) -> Option<&str> {
        let program_id = self.instruction.as_ref()?.program_id;
        let code = self.error_code()?;
        self.error_names.resolve(&program_id.to_string(), code)
    }

    /// Print the transaction logs. Consumes and returns `self`; chain or
    /// bind at chain end. Wrap in [`tap`](Self::tap) if you also want to
    /// inspect a borrowed view inside the same statement.
    pub fn print_logs(self) -> Self {
        print!("{}", self.tree_string());
        self
    }

    /// Same content as [`print_logs`](Self::print_logs) but returned as a
    /// `String` instead of printed.
    ///
    /// Substitutes pubkey aliases (well-known programs are included by
    /// `Aliases::default()`; user-named actors via `.with(pubkey, name)`)
    /// into each log line when an alias table is attached (see
    /// [`with_aliases`](Self::with_aliases)); falls back to
    /// [`Aliases::default`] otherwise.

    /// Swap the captured logs, so tree rendering is testable against a
    /// hand-written CPI stream without deploying a program.
    #[cfg(test)]
    pub(crate) fn set_logs_for_test(&mut self, logs: Vec<String>) {
        self.inner.logs = logs;
    }

    /// The error-code name from an `AnchorError` log line, if the run
    /// produced one. Anchor spells the name out
    /// (`... Error Code: SlippageExceeded. Error Number: 6009. ...`), so
    /// the failure can render by name even when the code is a framework
    /// error no IDL table registers.
    fn anchor_error_code(&self) -> Option<String> {
        self.inner.logs.iter().find_map(|log| {
            let rest = log.split("Error Code: ").nth(1)?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty()).then_some(name)
        })
    }

    /// The run as a CPI tree: one line per frame with the program (and,
    /// when the logs name it, instruction) label, invoke depth, outcome
    /// mark, and per-frame compute units; transaction signers annotate the
    /// top-level frames; a failed frame carries its resolved error as a
    /// leaf; decoded events render as badges inside their frame; and a
    /// legend maps every test-registered name back to its address. Falls
    /// back to [`logs_string`](Self::logs_string) when the logs yield no
    /// frames (a native-only transaction).
    pub fn tree_string(&self) -> String {
        use std::fmt::Write as _;
        let aliases_borrow;
        let aliases: &Aliases = match &self.aliases {
            Some(a) => a,
            None => {
                aliases_borrow = Aliases::default();
                &aliases_borrow
            }
        };
        let frames = anchor_litesvm_compat::cpi_tree(&self.inner.logs);
        if frames.is_empty() {
            return self.logs_string();
        }

        let mut render = TreeRender {
            aliases,
            well_known: Aliases::default(),
            legend: Vec::new(),
            signer_names: Vec::new(),
            error_leaf: self
                .anchor_error_code()
                .or_else(|| self.resolved_error_name().map(str::to_string))
                .or_else(|| self.error.clone()),
            events: &self.event_registry,
        };
        let signer_count = self.message.header.num_required_signatures as usize;
        let mut signer_names = Vec::new();
        for key in self.message.account_keys.iter().take(signer_count) {
            signer_names.push(render.label(key));
        }
        render.signer_names = signer_names;

        let top_title = {
            let f = &frames[0];
            let mut t = render.label(&f.program_id);
            if let Some(ix) = &f.instruction_name {
                t = format!("{t}::{ix}");
            }
            t
        };

        let mut out = String::new();
        writeln!(out).unwrap();
        let bar = "\u{2500}".repeat(60usize.saturating_sub(top_title.len() + 4));
        writeln!(out, "\u{2500}\u{2500} {top_title} {bar}").unwrap();
        writeln!(
            out,
            "Transaction  signers=[{}]",
            render.signer_names.join(", ")
        )
        .unwrap();

        let n = frames.len();
        for (i, f) in frames.iter().enumerate() {
            render.write_frame(&mut out, f, "", i == n - 1, 1);
        }

        if let Some(err) = &self.error {
            writeln!(out, "Error: {err}").unwrap();
        }
        writeln!(out, "Compute Units (this run): {}", self.compute_units()).unwrap();
        writeln!(out, "Fee: {} lamports", self.fee()).unwrap();
        if !render.legend.is_empty() {
            writeln!(out, "Legend ({}):", render.legend.len()).unwrap();
            let width = render
                .legend
                .iter()
                .map(|(n, _)| n.len())
                .max()
                .unwrap_or(0);
            for (name, pk) in &render.legend {
                writeln!(out, "  {name:width$} = {pk}").unwrap();
            }
        }
        out
    }

    pub fn logs_string(&self) -> String {
        use std::fmt::Write as _;
        let aliases_borrow;
        let aliases: &Aliases = match &self.aliases {
            Some(a) => a,
            None => {
                aliases_borrow = Aliases::default();
                &aliases_borrow
            }
        };
        let mut out = String::new();
        // Leading blank line separates our banner from whatever the test
        // runner just printed (the test name, prior assertions, etc.).
        writeln!(out).unwrap();
        writeln!(out, "=== Transaction Logs ===").unwrap();
        if let Some(info) = &self.instruction {
            writeln!(out, "Program: {}", aliases.label(&info.program_id)).unwrap();
        }
        for log in &self.inner.logs {
            writeln!(
                out,
                "{}",
                render_log_line(log, &self.event_registry, aliases)
            )
            .unwrap();
        }
        if let Some(err) = &self.error {
            writeln!(out, "Error: {}", err).unwrap();
        }
        // `(this run)` reminds readers the value is exact for *this*
        // execution; per-frame CU drifts across runs because Anchor's
        // find_program_address iterates a different number of bumps for
        // different random pubkeys.
        writeln!(out, "Compute Units (this run): {}", self.compute_units()).unwrap();
        writeln!(out, "Fee: {} lamports", self.fee()).unwrap();
        writeln!(out, "========================").unwrap();
        out
    }

    /// Get the inner TransactionMetadata for direct access
    pub fn inner(&self) -> &TransactionMetadata {
        &self.inner
    }

    /// The sent transaction's `Message`, for callers deriving their own
    /// signer/authority annotations from the account keys and header.
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Execute a closure for side effects on a borrowed view, then return
    /// the owned `self` for further chaining.
    ///
    /// This is the bridge between read-only methods (`compute_units`,
    /// `is_success`, `error`, `logs`, etc., which take `&self`) and the
    /// consuming chain methods (`assert_*`, `print_logs`). The closure
    /// receives `&Self`, so it can call any number of read-only methods
    /// inline; `tap` then hands ownership back to the chain.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::{Aliases, TransactionHelpers};
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let payer = Keypair::new();
    /// # let aliases = Aliases::default();
    /// let result = svm.send_ok(ix, &[&payer], &aliases)
    ///     .tap(|r| println!("CU used: {}", r.compute_units()))
    ///     .assert_success()
    ///     .print_logs();
    /// ```
    pub fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&Self),
    {
        f(&self);
        self
    }

    /// Assert that the transaction failed. Panics if it succeeded.
    /// Consumes and returns `self`; chain or bind at chain end.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::TransactionHelpers;
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let payer = Keypair::new();
    /// let result = svm.send_instruction(ix, &[&payer]).unwrap().assert_failure();
    /// ```
    pub fn assert_failure(self) -> Self {
        assert!(
            self.error.is_some(),
            "Expected transaction to fail, but it succeeded.\nLogs:\n{}",
            self.logs().join("\n")
        );
        self
    }

    /// Assert that the transaction failed AND a caller-supplied
    /// predicate holds on the result. Panics with context on either
    /// failure. Consumes and returns `self`.
    ///
    /// Mirrors [`assert_success_with`](Self::assert_success_with) for
    /// the negative-path case: useful for tests that want to verify
    /// both the failure and a specific signal about how/why it failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::TransactionHelpers;
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let payer = Keypair::new();
    /// let r = svm.send_instruction(ix, &[&payer]).unwrap()
    ///     .assert_failure_with(|r| r.has_log("EscrowExpired"));
    /// ```
    pub fn assert_failure_with<F>(self, predicate: F) -> Self
    where
        F: FnOnce(&Self) -> bool,
    {
        assert!(
            self.error.is_some(),
            "Expected transaction to fail, but it succeeded.\nLogs:\n{}",
            self.logs().join("\n")
        );
        assert!(
            predicate(&self),
            "Predicate failed on failed transaction.\nError: {:?}\nLogs:\n{}",
            self.error,
            self.logs().join("\n")
        );
        self
    }

    /// Assert that the transaction failed and the given substring
    /// appears in the runtime logs, the error field, or (for a
    /// single-instruction send) the name [`ErrorNames`] resolves the
    /// logged custom error code to. Panics if it succeeded or the
    /// substring wasn't found anywhere. Consumes and returns `self`.
    ///
    /// The lenient logs-or-error search covers Anchor error names
    /// (which surface as `Error code: <Name>` in logs but rarely in the
    /// error field), runtime errors (which surface in the error field,
    /// e.g. `"InsufficientFundsForRent"`), and programs without an IDL
    /// whose logs only carry the bare `0x<code>` (matched by name once
    /// registered via [`with_error_names`](Self::with_error_names)) with
    /// one method, so the caller doesn't have to remember which source
    /// carries which kind of error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = svm.send_instruction(ix, &[&payer])?
    ///     .assert_error("EscrowExpired");  // Anchor error name, in logs
    ///
    /// let result = svm.send_instruction(ix, &[&payer])?
    ///     .assert_error("InsufficientFundsForRent");  // runtime error, in the error field
    /// ```
    pub fn assert_error(self, expected_error: &str) -> Self {
        // assert_failure consumes; rebind so we can keep inspecting.
        let this = self.assert_failure();

        let found_in_logs = this.logs().iter().any(|log| log.contains(expected_error));
        let found_in_error = this
            .error
            .as_ref()
            .map(|e| e.contains(expected_error))
            .unwrap_or(false);
        let found_in_resolved = this
            .resolved_error_name()
            .map(|name| name.contains(expected_error))
            .unwrap_or(false);

        assert!(
            found_in_logs || found_in_error || found_in_resolved,
            "Expected error containing '{}' not found in transaction logs, error field, or resolved failure name.\nError: {:?}\nLogs:\n{}",
            expected_error,
            this.error,
            this.logs().join("\n")
        );
        this
    }

    /// Assert that the transaction failed with a specific Anchor custom
    /// error code (e.g. `6000` for the first error in an Anchor
    /// `#[error_code]` enum). Formats the code as the
    /// `"custom program error: 0x<hex>"` substring runtime emits, then
    /// delegates to [`assert_error`](Self::assert_error). Consumes and
    /// returns `self`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::TransactionHelpers;
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let payer = Keypair::new();
    /// let result = svm.send_instruction(ix, &[&payer]).unwrap()
    ///     .assert_error_code(6000);
    /// ```
    pub fn assert_error_code(self, error_code: u32) -> Self {
        let error_code_str = format!("custom program error: 0x{:x}", error_code);
        self.assert_error(&error_code_str)
    }
}

impl fmt::Debug for TransactionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransactionResult")
            .field(
                "program_id",
                &self.instruction.as_ref().map(|i| i.program_id),
            )
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
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let signer = Keypair::new();
    /// let result = svm.send_instruction(ix, &[&signer]).unwrap().assert_success();
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
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix1 = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let ix2 = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let signer = Keypair::new();
    /// let result = svm.send_instructions(&[ix1, ix2], &[&signer]).unwrap().assert_success();
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
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # use litesvm_utils::Signer;
    /// # use litesvm_utils::Transaction;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let signer = Keypair::new();
    /// let tx = Transaction::new_signed_with_payer(
    ///     &[ix],
    ///     Some(&signer.pubkey()),
    ///     &[&signer],
    ///     svm.latest_blockhash(),
    /// );
    /// let result = svm.send_transaction_result(tx).unwrap().assert_success();
    /// ```
    fn send_transaction_result(
        &mut self,
        transaction: Transaction,
    ) -> Result<TransactionResult, TransactionError>;

    /// Send an ix expected to succeed: unwraps the build-time `Result`
    /// (panics on build errors, e.g. no signers) and asserts the
    /// transaction itself didn't carry a program error. Returns the
    /// wrapped result with `aliases` stashed on it, so callers can chain
    /// `.print_logs()` or inspect compute units.
    ///
    /// The `aliases` map is used for the failure-path log print, so test
    /// authors who built an alias map in setup see it applied to the
    /// diagnostic. Pass `&Aliases::default()` if you just want the
    /// well-known program names.
    ///
    /// Use this in the happy path of a test. When failure is expected
    /// with a specific Anchor error name, use
    /// [`send_err_named`](Self::send_err_named); when failure is
    /// expected without a name to assert (the outcome alone is the
    /// contract), use [`send_err`](Self::send_err); otherwise drop down
    /// to [`send_instruction`](Self::send_instruction).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::{Aliases, TransactionHelpers};
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let maker = Keypair::new();
    /// # let aliases = Aliases::default();
    /// svm.send_ok(ix, &[&maker], &aliases).print_logs();
    /// ```
    fn send_ok(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
        aliases: &Aliases,
    ) -> TransactionResult {
        let mut result = self
            .send_instruction(instruction, signers)
            .expect("send_ok: transaction build failed")
            .with_aliases(aliases.clone());
        if !result.is_success() {
            // The underlying assert_success panic includes only flat logs.
            // Print the aliased logs first so the test author sees which
            // program failed, then let assert_success raise the panic with
            // its embedded log dump as normal.
            eprintln!("\nsend_ok: transaction failed, logs:");
            result = result.print_logs();
        }
        result.assert_success()
    }

    /// Send an ix expected to fail without asserting a specific error
    /// name. Mirror of [`send_ok`](Self::send_ok) for the negative path
    /// when the outcome alone is the contract (e.g. an authorization
    /// check, a generic constraint trip) and pinning to a specific
    /// `ErrorCode::Foo` would over-constrain the test.
    ///
    /// The `aliases` map is used for the failure-path log print (i.e.
    /// when the tx *unexpectedly succeeded*, which is the
    /// assertion-failure mode here). Returns the wrapped result so
    /// callers can chain further inspection.
    ///
    /// Panics if the transaction succeeded.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::{Aliases, TransactionHelpers};
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let attacker = Keypair::new();
    /// # let aliases = Aliases::default();
    /// svm.send_err(ix, &[&attacker], &aliases)
    ///     .print_logs();
    /// ```
    fn send_err(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
        aliases: &Aliases,
    ) -> TransactionResult {
        let mut result = self
            .send_instruction(instruction, signers)
            .expect("send_err: transaction build failed")
            .with_aliases(aliases.clone());
        if result.is_success() {
            eprintln!("\nsend_err: tx unexpectedly succeeded, logs:");
            result = result.print_logs();
        }
        result.assert_failure()
    }

    /// Send an ix expected to fail with a specific error name
    /// (e.g. `"EscrowExpired"`, `"ConstraintHasOne"`,
    /// `"InsufficientFundsForRent"`). The match is the same substring
    /// check that [`TransactionResult::assert_error`] performs against
    /// logs and the error field, just bundled with the send + unwrap.
    /// Returns the wrapped result so callers can chain further
    /// inspection (compute units, log scrapes, custom prints),
    /// mirroring [`send_ok`](Self::send_ok)'s shape.
    ///
    /// The `aliases` map is used for the failure-path log print (same as
    /// [`send_ok`](Self::send_ok)). Pass `&Aliases::default()` if you
    /// just want the well-known program names.
    ///
    /// Panics if the transaction succeeded or failed with a different
    /// error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use litesvm_utils::{Aliases, TransactionHelpers};
    /// # use litesvm_utils::LiteSVM;
    /// # use solana_program::instruction::Instruction;
    /// # use litesvm_utils::Keypair;
    /// # let mut svm = LiteSVM::new();
    /// # let ix = Instruction::new_with_bytes(solana_program::pubkey::Pubkey::new_unique(), &[], vec![]);
    /// # let taker = Keypair::new();
    /// # let aliases = Aliases::default();
    /// svm.send_err_named(ix, &[&taker], &aliases, "EscrowExpired")
    ///     .tap(|r| assert!(r.compute_units() < 100_000));
    /// ```
    fn send_err_named(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
        aliases: &Aliases,
        error_name: &str,
    ) -> TransactionResult {
        let mut result = self
            .send_instruction(instruction, signers)
            .expect("send_err_named: transaction build failed")
            .with_aliases(aliases.clone());
        // If we're about to fail the assertion (tx succeeded, or tx failed
        // with a different error), print the logs first so they're visible
        // above the eventual panic dump.
        let error_matches = !result.is_success()
            && (result.logs().iter().any(|log| log.contains(error_name))
                || result
                    .error()
                    .map(|e| e.contains(error_name))
                    .unwrap_or(false));
        if !error_matches {
            eprintln!("\nsend_err_named: assertion will fail, logs:");
            result = result.print_logs();
        }
        result.assert_error(error_name)
    }
}

impl TransactionHelpers for LiteSVM {
    fn send_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError> {
        if signers.is_empty() {
            return Err(TransactionError::BuildError(
                "No signers provided".to_string(),
            ));
        }

        // We know this transaction wraps exactly one instruction, so we
        // capture its program ID + data for a later custom-error-code lookup.
        // Stashed before the `Transaction::new_signed_with_payer` call
        // below because that call consumes `instruction`.
        let info = InstructionInfo::from_instruction(&instruction);
        // Fresh by default: every helper-mediated send is its own transaction.
        // Without this, an identical instruction resent under the same
        // blockhash is the same signature, and litesvm correctly rejects the
        // repeat as already processed — a chain rule no test scenario means
        // to invoke (and one that is litesvm's own to test, via raw litesvm).
        self.expire_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&signers[0].pubkey()),
            signers,
            self.latest_blockhash(),
        );
        let message = tx.message.clone();
        match self.send_transaction(tx) {
            Ok(result) => Ok(TransactionResult::new(result, Some(info), message)),
            Err(failed) => Ok(TransactionResult::new_failed(
                format!("{:?}", failed.err),
                failed.meta,
                Some(info),
                message,
            )),
        }
    }

    fn send_instructions(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError> {
        if signers.is_empty() {
            return Err(TransactionError::BuildError(
                "No signers provided".to_string(),
            ));
        }

        // Fresh by default; see send_instruction for the full rationale.
        self.expire_blockhash();
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
        // Clone the message before send_transaction consumes the tx; kept
        // for parity with the single-instruction path even though no
        // consumer reads it back today.
        let message = transaction.message.clone();
        match self.send_transaction(transaction) {
            Ok(result) => Ok(TransactionResult::new(result, None, message)),
            Err(failed) => Ok(TransactionResult::new_failed(
                format!("{:?}", failed.err),
                failed.meta,
                None,
                message,
            )),
        }
    }
}

#[cfg(test)]
mod tests;
