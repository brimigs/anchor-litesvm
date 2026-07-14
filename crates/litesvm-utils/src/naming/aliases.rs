//! User-extensible pubkey-to-friendly-name resolver.
//!
//! Substitutes readable names for program IDs and signer pubkeys at render
//! time. Owns the static list of well-known Solana programs (System, Token,
//! etc.) as a baseline; user-added aliases take precedence (last insert
//! wins).

use {
    solana_program::pubkey::Pubkey,
    std::{collections::HashMap, str::FromStr},
};

/// Well-known Solana program IDs that `Aliases::with_well_known()`
/// pre-seeds. This list previously lived in `tree.rs`.
const WELL_KNOWN_PROGRAMS: &[(&str, &str)] = &[
    ("11111111111111111111111111111111", "System"),
    ("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "Token"),
    ("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", "Token-2022"),
    (
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
        "AssociatedToken",
    ),
    (
        "ComputeBudget111111111111111111111111111111",
        "ComputeBudget",
    ),
    (
        "BPFLoaderUpgradeab1e11111111111111111111111",
        "BPFLoaderUpgradeable",
    ),
    ("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr", "Memo"),
    ("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo", "Memo-v1"),
];

/// Pubkey-to-friendly-name resolver for the structured-logs printer.
///
/// `default()` (alias for `with_well_known()`) seeds with the well-known
/// Solana programs (System, Token, etc.). Extend with `.with(pubkey,
/// name)` to name your own actors and programs.
#[derive(Debug, Clone)]
pub struct Aliases {
    by_pubkey: HashMap<Pubkey, String>,
    /// Names handed to [`ActorRegistry::create`](super::actors::ActorRegistry::create).
    /// Each cast seeds a keypair from its name and registers an alias, so a
    /// repeated name silently forks one identity in two;
    /// [`register_cast`](Self::register_cast) is the guard. Distinct from
    /// `by_pubkey`, which also holds non-cast aliases (PDAs, programs) and is
    /// last-write-wins for renames.
    cast_names: std::collections::HashSet<String>,
}

impl Aliases {
    /// Resolver pre-seeded with `WELL_KNOWN_PROGRAMS`.
    pub fn with_well_known() -> Self {
        let mut a = Self {
            by_pubkey: HashMap::new(),
            cast_names: std::collections::HashSet::new(),
        };
        for (id_str, name) in WELL_KNOWN_PROGRAMS {
            let pk = Pubkey::from_str(id_str).expect("WELL_KNOWN_PROGRAMS ID is valid base58");
            a.by_pubkey.insert(pk, (*name).to_string());
        }
        a
    }

    /// Chainable insert. Later inserts shadow earlier ones, so a user
    /// alias for a well-known program ID overrides the built-in name.
    pub fn with(mut self, pubkey: Pubkey, name: impl Into<String>) -> Self {
        self.add(pubkey, name);
        self
    }

    /// In-place insert. The accumulation companion to [`with`](Self::with):
    /// use `with` to chain on a fresh `Aliases` value (the builder seed),
    /// use `add` when the table lives behind `&mut self` and needs to grow
    /// over time (the scenario-table case).
    pub fn add(&mut self, pubkey: Pubkey, name: impl Into<String>) -> &mut Self {
        self.by_pubkey.insert(pubkey, name.into());
        self
    }

    /// Record `name` as a freshly cast identity, returning `false` if it was
    /// already cast on this table. The cast-list discipline: because a cast
    /// seeds its keypair from its name, casting one name twice hands back the
    /// same address (and re-funds it), almost always a mistake. The cast
    /// helpers assert on this; non-cast aliases ([`add`](Self::add)) are
    /// unaffected and stay last-write-wins.
    pub fn register_cast(&mut self, name: &str) -> bool {
        self.cast_names.insert(name.to_string())
    }

    /// Look up a name for a `Pubkey`. Used by `LegendCollector` while
    /// rendering program IDs and signer pubkeys; also exposed for
    /// callers that want to verify aliasing state directly.
    pub fn resolve_by_pubkey(&self, pubkey: &Pubkey) -> Option<&str> {
        self.by_pubkey.get(pubkey).map(String::as_str)
    }

    /// The friendly name registered for `pubkey`, or a short `<8>…<4>`
    /// truncation of the raw key when it isn't aliased.
    ///
    /// Built for report rows (before/after tables and the like): alias
    /// the accounts you want named, and anything you miss still renders
    /// compactly and identically to how the structured tree shows it.
    /// Returns an owned `String` so it drops straight into a
    /// `MarkdownBlock` cell.
    pub fn label(&self, pubkey: &Pubkey) -> String {
        self.resolve_by_pubkey(pubkey)
            .map(str::to_string)
            .unwrap_or_else(|| short_pubkey(pubkey))
    }

    /// Replace every registered key's base58 string with its alias name,
    /// wherever it appears in `text`.
    ///
    /// Built for free-form text the model can't pre-resolve into `Pubkey`s: a
    /// decoded event's `{:?}` field body, where a key arrives as a base58
    /// substring (`from: 5xY8...`) rather than a typed `Pubkey` the renderer
    /// could `label`. A base58 key is 32+ unique characters, so a plain
    /// substring replace can't collide with anything else in the text.
    pub fn substitute_in_text(&self, text: &str) -> String {
        let mut out = text.to_string();
        for (pubkey, name) in &self.by_pubkey {
            out = out.replace(&pubkey.to_string(), name);
        }
        out
    }

    /// Every `(pubkey, name)` pair, for serializing the table into a report
    /// record so the Reporter can map addresses to roles offline.
    pub fn entries(&self) -> Vec<(Pubkey, String)> {
        self.by_pubkey
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }
}

/// The naming capability every consumer of rendered output depends on: turn a
/// pubkey into its display form, and resolve the pubkeys embedded in a piece
/// of free text.
///
/// It is a named contract rather than a closure threaded here and a
/// `substitute_in_text` call reached for there: a consumer takes
/// `&dyn Labeler` and never has to know whether the names come from an
/// [`Aliases`] table or anything else. [`Aliases`] is the production impl;
/// [`RawLabeler`] is the no-aliases identity.
///
/// Two methods because there are two shapes of value: a typed program id is a
/// `Pubkey` ([`label`](Labeler::label)); a decoded event field is free text
/// that may carry a base58 key as a substring
/// ([`substitute_in_text`](Labeler::substitute_in_text)).
pub trait Labeler {
    /// The display form of `pubkey`: its registered name, or a short fallback.
    fn label(&self, pubkey: &Pubkey) -> String;
    /// The text with every embedded base58 key replaced by its name.
    fn substitute_in_text(&self, text: &str) -> String;
}

impl Labeler for Aliases {
    fn label(&self, pubkey: &Pubkey) -> String {
        Aliases::label(self, pubkey)
    }
    fn substitute_in_text(&self, text: &str) -> String {
        Aliases::substitute_in_text(self, text)
    }
}

/// The identity labeler: a pubkey renders as its base58, free text passes
/// through unchanged. The fallback when no alias table is in play.
pub struct RawLabeler;

impl Labeler for RawLabeler {
    fn label(&self, pubkey: &Pubkey) -> String {
        pubkey.to_string()
    }
    fn substitute_in_text(&self, text: &str) -> String {
        text.to_string()
    }
}

/// `<first 8>…<last 4>` of the base58 key (keys of 12 chars or fewer are
/// left whole). Shared by the structured-tree renderer and
/// [`Aliases::label`] so an unaliased key reads the same everywhere.
pub fn short_pubkey(pubkey: &Pubkey) -> String {
    let s = pubkey.to_string();
    if s.len() > 12 {
        format!("{}…{}", &s[..8], &s[s.len() - 4..])
    } else {
        s
    }
}

/// Whether `name` matches one of the friendly names seeded by
/// [`Aliases::with_well_known`]. Used by the structured-logs printer to
/// keep the legend footer focused on user-supplied actors and programs
/// (System, Token, AssociatedToken, etc. are noise in a test log; they're
/// already documented elsewhere). A user-renamed well-known program
/// (e.g. `.with(system_pk, "MyNamedSystem")`) does *not* match here, so
/// such renames surface in the legend as the user clearly intended.
pub fn is_well_known_name(name: &str) -> bool {
    WELL_KNOWN_PROGRAMS.iter().any(|(_, n)| *n == name)
}

impl Default for Aliases {
    fn default() -> Self {
        Self::with_well_known()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_well_known_seeds_system_and_token() {
        let a = Aliases::with_well_known();
        let system_pk = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        let token_pk = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        assert_eq!(a.resolve_by_pubkey(&system_pk), Some("System"));
        assert_eq!(a.resolve_by_pubkey(&token_pk), Some("Token"));
    }

    #[test]
    fn default_matches_with_well_known() {
        let d: Aliases = Default::default();
        let system_pk = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        assert_eq!(d.resolve_by_pubkey(&system_pk), Some("System"));
    }

    #[test]
    fn label_uses_alias_then_falls_back_to_short_key() {
        let aliased = Pubkey::new_unique();
        let a = Aliases::default().with(aliased, "maker");
        // Aliased key resolves to its name.
        assert_eq!(a.label(&aliased), "maker");
        // Unaliased key truncates to `<8>…<4>` (44-char base58 keys are
        // always longer than the 12-char keep-whole threshold).
        let bare = Pubkey::new_unique();
        let s = bare.to_string();
        assert_eq!(a.label(&bare), format!("{}…{}", &s[..8], &s[s.len() - 4..]));
    }

    #[test]
    fn with_inserts_user_alias() {
        let pk = Pubkey::new_unique();
        let a = Aliases::with_well_known().with(pk, "alice");
        assert_eq!(a.resolve_by_pubkey(&pk), Some("alice"));
    }

    #[test]
    fn later_with_shadows_earlier() {
        let system_pk = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        let a = Aliases::with_well_known().with(system_pk, "MyNamedSystem");
        assert_eq!(a.resolve_by_pubkey(&system_pk), Some("MyNamedSystem"));
    }

    #[test]
    fn with_chains() {
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        let a = Aliases::with_well_known().with(pk1, "one").with(pk2, "two");
        assert_eq!(a.resolve_by_pubkey(&pk1), Some("one"));
        assert_eq!(a.resolve_by_pubkey(&pk2), Some("two"));
    }

    #[test]
    fn add_inserts_in_place_and_returns_mut_for_chaining() {
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        let mut a = Aliases::with_well_known();
        a.add(pk1, "one").add(pk2, "two");
        assert_eq!(a.resolve_by_pubkey(&pk1), Some("one"));
        assert_eq!(a.resolve_by_pubkey(&pk2), Some("two"));
    }

    #[test]
    fn add_later_inserts_shadow_earlier() {
        let pk = Pubkey::new_unique();
        let mut a = Aliases::with_well_known();
        a.add(pk, "first");
        a.add(pk, "second");
        assert_eq!(a.resolve_by_pubkey(&pk), Some("second"));
    }
}
