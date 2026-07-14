//! Deterministic test identities, Ganache/Hardhat style.
//!
//! Random `Keypair::new()` makes every run's addresses different, which is fine
//! until you want to *commit* test output (structured logs, saved fixtures): the
//! base58 keys churn run to run and every diff is noise. Seeding keypairs from a
//! fixed domain + role string fixes that: the same role always yields the same
//! pubkey, so addresses (and anything derived from them: PDAs, ATAs) are stable
//! forever, and the output diffs cleanly.
//!
//! Two layers:
//!   - [`deterministic_keypair`] / [`seed_bytes`]: the bare derivation.
//!   - [`ActorRegistry`]: tracks which role labels a scenario has used, so a
//!     duplicate label (which would silently alias two "actors" to one address,
//!     since the label *is* the seed) becomes an immediate panic, with an escape
//!     hatch for legitimately re-fetching an existing identity.

use anchor_litesvm_compat::Keypair;
use std::collections::HashSet;

/// Derive a deterministic ed25519 keypair from a `(domain, role)` pair.
///
/// `domain` is your project's namespace (think Ganache mnemonic): keep it
/// constant and every identity is stable across runs; bump it (e.g. `myapp/v1`
/// -> `myapp/v2`) to deliberately reshuffle the whole address space. `role` is
/// the identity within that namespace ("authority", "actor:Alice", "mint:x").
///
/// Keying off a *name* rather than an index means reordering your cast doesn't
/// shift anyone's address, which is what keeps committed output stable when you
/// add or move tests.
pub fn deterministic_keypair(domain: &str, role: &str) -> Keypair {
    Keypair::new_from_array(seed_bytes(&format!("{domain}:{role}")))
}

/// Deterministic 32 bytes from a string, in two stages:
///
/// 1. **FNV-1a (64-bit)** folds the whole input down to a single 64-bit hash.
///    FNV-1a is a tiny, well-known non-cryptographic string hash; spec and the
///    canonical constants are at <https://www.rfc-editor.org/info/rfc9923/>.
/// 2. **splitmix64** then treats that hash as a seed and emits four successive
///    64-bit outputs, one per 8-byte lane, to fill the 32-byte array. splitmix64
///    is the small fixed-increment generator from Vigna's SplittableRandom work;
///    reference implementation at <https://prng.di.unimi.it/splitmix64.c>. It is
///    used here purely as a seed *expander* (64 bits -> 256 bits), not as a PRNG
///    anyone draws from over time.
///
/// Every constant below is the published value for its algorithm; nothing here is
/// invented, so two readers can check the numbers against the references rather
/// than trust them. Pure, allocation-free, dependency-free, and stable forever,
/// so derived pubkeys never move between runs or toolchains.
///
/// No cryptographic strength is claimed or needed: these are throwaway test keys,
/// never funded on any real network. All we need is determinism and
/// collision-freedom across distinct input strings, which this trivially
/// provides. The namespace of test labels is intentionally small enough that a
/// 64-bit FNV seed has ample room before birthday collisions become plausible
/// (~2^32 distinct labels), so the single 64-bit fold is more than sufficient.
/// (We hash inline rather than pull in sha2 to keep the crate's dependency
/// surface and build immune to hashing-crate churn.)
pub fn seed_bytes(input: &str) -> [u8; 32] {
    // --- Stage 1: FNV-1a 64-bit fold (input string -> one u64) ---
    // FNV-1a: for each byte, XOR into the hash, then multiply by the prime.
    // FNV64basis (0xcbf29ce484222325) = the 64-bit FNV published start value.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in input.bytes() {
        h ^= b as u64;
        // 0x100000001b3 = the 64-bit FNV prime. wrapping_mul = the mod-2^64
        // arithmetic FNV is defined over.
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }

    // --- Stage 2: splitmix64 expansion (one u64 seed -> 32 bytes) ---
    let mut out = [0u8; 32];
    let mut x = h;
    for lane in out.chunks_mut(8) {
        // 0x9e3779b97f4a7c15 = splitmix64's golden-ratio increment ("gamma"):
        // advance the state by it before each output so successive lanes differ.
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        // The splitmix64 finalizer: two xor-shift-multiply rounds plus a final
        // xor-shift. The two odd multipliers and the 30/27/31 shift widths are
        // the published constants, chosen to give good avalanche (each input bit
        // affects roughly half the output bits).
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        // Little-endian is an arbitrary-but-fixed choice; it only has to be
        // consistent across runs, which `to_le_bytes` guarantees on every target.
        lane.copy_from_slice(&z.to_le_bytes());
    }
    out
}

/// Tracks the role labels used in one scenario and enforces "one label = one
/// identity".
///
/// Because [`deterministic_keypair`] derives the key *from* the label, two
/// actors sharing a label would silently get the same address, and every
/// assertion past that point would reason about the wrong account. The registry
/// turns that footgun into an immediate panic: [`create`](Self::create) is the
/// unique "mint this identity" call (and fails loudly on a repeat), while
/// [`get`](Self::get) is the escape hatch for a *second handle* to an identity
/// that already exists. This mirrors Rust's own aliasing rule: `create` is the
/// unique `&mut` that brings a thing into being; `get` is a shared `&` to
/// something already there (note it takes `&self`).
///
/// The registry only manages keypairs and labels; it knows nothing about token
/// accounts or funding. Wrap it in your harness's scenario type and layer those
/// concerns (airdrop, ATAs, alias-table registration) on top of `create`/`get`.
pub struct ActorRegistry {
    domain: String,
    used: HashSet<String>,
}

impl ActorRegistry {
    /// Create an empty registry namespaced by `domain` (see
    /// [`deterministic_keypair`]).
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            used: HashSet::new(),
        }
    }

    /// Mint the identity for `label`: derive its keypair, record the label, and
    /// return the keypair. Panics if `label` was already created in this
    /// registry (a duplicate would alias two actors to one address).
    pub fn create(&mut self, label: &str) -> Keypair {
        assert!(
            self.used.insert(label.to_string()),
            "actor label {label:?} already used in this scenario; labels seed \
             keypairs, so a duplicate silently aliases two actors to one \
             address. Give this actor a distinct label, or use `get` to re-fetch \
             an existing one."
        );
        self.keypair(label)
    }

    /// Escape hatch: re-derive a *second handle* to an actor that already
    /// exists, without recording a new label. Panics if `label` was never
    /// created (re-fetching a nonexistent actor is the real mistake). Use this
    /// when a helper needs another reference to an identity, or one actor plays
    /// two narrative roles.
    pub fn get(&self, label: &str) -> Keypair {
        assert!(
            self.used.contains(label),
            "no actor labelled {label:?} exists yet; `get` only re-derives a \
             handle to one already created via `create`. Call `create` first."
        );
        self.keypair(label)
    }

    /// Derive a keypair for a non-actor role in this domain (e.g. a mint or a
    /// global authority), bypassing the label registry. These don't participate
    /// in actor-label uniqueness, so they're not recorded.
    pub fn keypair(&self, role: &str) -> Keypair {
        deterministic_keypair(&self.domain, role)
    }

    /// Whether `label` has been created in this registry.
    pub fn contains(&self, label: &str) -> bool {
        self.used.contains(label)
    }

    /// The namespace this registry derives within.
    pub fn domain(&self) -> &str {
        &self.domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_litesvm_compat::Signer;

    #[test]
    fn same_role_same_key_across_calls() {
        let a = deterministic_keypair("d/v1", "actor:Alice");
        let b = deterministic_keypair("d/v1", "actor:Alice");
        assert_eq!(a.pubkey(), b.pubkey());
    }

    #[test]
    fn different_domain_or_role_differs() {
        let base = deterministic_keypair("d/v1", "actor:Alice").pubkey();
        assert_ne!(base, deterministic_keypair("d/v2", "actor:Alice").pubkey());
        assert_ne!(base, deterministic_keypair("d/v1", "actor:Bob").pubkey());
    }

    #[test]
    #[should_panic(expected = "already used in this scenario")]
    fn duplicate_create_panics() {
        let mut r = ActorRegistry::new("d/v1");
        r.create("Alice");
        r.create("Alice");
    }

    #[test]
    fn get_refetches_same_identity() {
        let mut r = ActorRegistry::new("d/v1");
        let first = r.create("Alice").pubkey();
        assert_eq!(first, r.get("Alice").pubkey());
    }

    #[test]
    #[should_panic(expected = "exists yet")]
    fn get_unknown_panics() {
        let r = ActorRegistry::new("d/v1");
        r.get("Ghost");
    }
}
