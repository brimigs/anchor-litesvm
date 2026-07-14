//! Fabrication of Metaplex Token Metadata accounts by writing their bytes.
//!
//! Hand-serialized on purpose: depending on `mpl-token-metadata` drags a second
//! `solana-program` major into the host build (it pins v2 against our v3), and a
//! test fabricator only needs to emit bytes the program-under-test deserializes.
//!
//! This is a *maintained compatibility surface*, not a shortcut: the framework
//! owns the layout so a consumer fabricates an NFT in one call without an mpl
//! dependency. Token Metadata is append-only stable (fields are added to the
//! tail; the prefix and the `key = 4` discriminator never move), so the
//! maintenance burden is low and back-compat is structural.
//!
//! The *full* current struct is serialized (through `programmable_config`). The
//! commonly-gated fields (collection, creators, token_standard) are settable;
//! the rest are written as `None` so any reader gets a valid value rather than
//! running off the end.

use anchor_litesvm_compat::{Account, LiteSVM};
use solana_program::pubkey::Pubkey;

/// The Metaplex Token Metadata program id.
pub const MPL_TOKEN_METADATA_ID: Pubkey =
    Pubkey::from_str_const("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
/// The metadata PDA seed prefix: `["metadata", program_id, mint]`.
pub const METADATA_SEED: &[u8] = b"metadata";

/// Metaplex `TokenStandard` discriminant, the metadata's version marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenStandard {
    NonFungible = 0,
    FungibleAsset = 1,
    Fungible = 2,
    NonFungibleEdition = 3,
    ProgrammableNonFungible = 4,
    ProgrammableNonFungibleEdition = 5,
}

/// A Metaplex creator entry (`address` + `verified` + `share`).
#[derive(Clone, Copy, Debug)]
pub struct Creator {
    pub address: Pubkey,
    pub verified: bool,
    pub share: u8,
}

/// Fields for a fabricated metadata account. Commonly-gated fields are settable;
/// trailing optional fields (uses, collection_details, programmable_config) are
/// always written as `None`.
#[derive(Clone, Debug)]
pub struct MetadataArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    /// Empty = serialized as `None`.
    pub creators: Vec<Creator>,
    /// `(collection_key, verified)`; `None` = no collection.
    pub collection: Option<(Pubkey, bool)>,
    pub token_standard: Option<TokenStandard>,
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
}

impl Default for MetadataArgs {
    fn default() -> Self {
        Self {
            name: "NFT".into(),
            symbol: "NFT".into(),
            uri: "x".into(),
            seller_fee_basis_points: 0,
            creators: Vec::new(),
            collection: None,
            token_standard: Some(TokenStandard::NonFungible),
            primary_sale_happened: false,
            is_mutable: true,
        }
    }
}

/// Fabricate Metaplex metadata accounts by writing their bytes directly.
pub trait MetaplexHelpers {
    /// The Metaplex Token Metadata PDA for `mint`.
    fn metadata_pda(&self, mint: &Pubkey) -> Pubkey;

    /// Fabricate a Metaplex Token Metadata account for `mint`, owned by the
    /// Token Metadata program at its canonical PDA. Returns the PDA.
    fn fabricate_metadata(&mut self, mint: &Pubkey, args: &MetadataArgs) -> Pubkey;
}

impl MetaplexHelpers for LiteSVM {
    fn metadata_pda(&self, mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[METADATA_SEED, MPL_TOKEN_METADATA_ID.as_ref(), mint.as_ref()],
            &MPL_TOKEN_METADATA_ID,
        )
        .0
    }

    fn fabricate_metadata(&mut self, mint: &Pubkey, args: &MetadataArgs) -> Pubkey {
        let pda = self.metadata_pda(mint);
        let mut v = Vec::new();
        v.push(4u8); // key = MetadataV1
        v.extend_from_slice(&[0u8; 32]); // update_authority
        v.extend_from_slice(mint.as_ref()); // mint
                                            // Data { name, symbol, uri, seller_fee_basis_points, creators }
        push_string(&mut v, &args.name);
        push_string(&mut v, &args.symbol);
        push_string(&mut v, &args.uri);
        v.extend_from_slice(&args.seller_fee_basis_points.to_le_bytes());
        push_creators(&mut v, &args.creators);
        // tail
        v.push(args.primary_sale_happened as u8);
        v.push(args.is_mutable as u8);
        v.push(0); // edition_nonce: None
        push_token_standard(&mut v, args.token_standard);
        push_collection(&mut v, args.collection);
        v.push(0); // uses: None
        v.push(0); // collection_details: None
        v.push(0); // programmable_config: None
        let rent = self.minimum_balance_for_rent_exemption(v.len());
        self.set_account(
            pda,
            Account {
                lamports: rent,
                data: v,
                owner: MPL_TOKEN_METADATA_ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .expect("set fabricated metadata");
        pda
    }
}

fn push_string(v: &mut Vec<u8>, s: &str) {
    v.extend_from_slice(&(s.len() as u32).to_le_bytes());
    v.extend_from_slice(s.as_bytes());
}

// creators: Option<Vec<Creator>>, Creator = pubkey(32) + verified(1) + share(1)
fn push_creators(v: &mut Vec<u8>, creators: &[Creator]) {
    if creators.is_empty() {
        v.push(0); // None
        return;
    }
    v.push(1); // Some
    v.extend_from_slice(&(creators.len() as u32).to_le_bytes());
    for c in creators {
        v.extend_from_slice(c.address.as_ref());
        v.push(c.verified as u8);
        v.push(c.share);
    }
}

fn push_token_standard(v: &mut Vec<u8>, ts: Option<TokenStandard>) {
    match ts {
        Some(s) => {
            v.push(1);
            v.push(s as u8);
        }
        None => v.push(0),
    }
}

// collection: Option<Collection { verified: bool, key: Pubkey }>
fn push_collection(v: &mut Vec<u8>, collection: Option<(Pubkey, bool)>) {
    match collection {
        Some((key, verified)) => {
            v.push(1);
            v.push(verified as u8);
            v.extend_from_slice(key.as_ref());
        }
        None => v.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabricated_metadata_is_full_fidelity_owned_pda() {
        let mut svm = LiteSVM::new();
        let mint = Pubkey::new_unique();
        let collection = Pubkey::new_from_array([0xAB; 32]);
        let pda = svm.fabricate_metadata(
            &mint,
            &MetadataArgs {
                collection: Some((collection, true)),
                ..Default::default()
            },
        );
        assert_eq!(pda, svm.metadata_pda(&mint));
        let acct = svm.get_account(&pda).expect("metadata exists");
        assert_eq!(acct.owner, MPL_TOKEN_METADATA_ID);
        assert_eq!(acct.data[0], 4); // MetadataV1 key
        assert_eq!(&acct.data[33..65], mint.as_ref()); // mint field (after key + update_authority)
                                                       // Full fidelity: the collection key precedes three trailing `None` bytes
                                                       // (uses, collection_details, programmable_config) at the end of the buffer.
        let n = acct.data.len();
        assert_eq!(&acct.data[n - 35..n - 3], collection.as_ref());
        assert_eq!(&acct.data[n - 3..], &[0u8, 0, 0]);
    }
}
