//! Direct *fabrication* of SPL / Token-2022 accounts by writing their bytes.
//!
//! Sibling to [`TestHelpers::create_token_mint`](crate::TestHelpers), which
//! initializes through a real token-program transaction. Fabrication is for when
//! a test just needs an account to *exist* with chosen contents (an NFT mint a
//! policy inspects, a holder with a balance) without minting it for real. It
//! uses the canonical `spl-token` `Mint` / `Account` layouts, so it tracks them
//! if they move.
//!
//! Classic SPL and Token-2022 share that *base* layout byte-for-byte; they
//! differ only in the owning program (and, for T22, an optional extension
//! trailer). So one fabricator parameterized by [`TokenProgram`] covers both
//! base cases. Token-2022 *extensions* will extend this module (they need
//! `spl-token-2022`, which resolves clean against our solana-program); the base
//! path here needs only the program id.

use anchor_litesvm_compat::{Account, LiteSVM};
use solana_program::program_option::COption;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use spl_token::state::{Account as TokenAccount, AccountState, Mint};

/// The Token-2022 (Token Extensions) program id.
pub const TOKEN_2022_ID: Pubkey =
    Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Which token program owns a fabricated account. The base account layout is
/// identical across the two; only the owner (and T22 extensions) differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenProgram {
    Spl,
    Token2022,
}

impl TokenProgram {
    /// The owning program id.
    pub fn id(self) -> Pubkey {
        match self {
            TokenProgram::Spl => spl_token::id(),
            TokenProgram::Token2022 => TOKEN_2022_ID,
        }
    }
}

/// Fabricate SPL / Token-2022 accounts by writing their bytes directly.
pub trait TokenFabrication {
    /// Fabricate an initialized base [`Mint`] at `mint` with the given `decimals`
    /// and `supply`, owned by `program`. No init transaction.
    fn fabricate_mint(&mut self, mint: &Pubkey, program: TokenProgram, decimals: u8, supply: u64);

    /// Fabricate an NFT mint: an initialized Mint with `supply == 1` and
    /// `decimals == 0`, the canonical "this is an NFT" shape, owned by `program`.
    fn fabricate_nft_mint(&mut self, mint: &Pubkey, program: TokenProgram) {
        self.fabricate_mint(mint, program, 0, 1);
    }

    /// Fabricate an initialized base token [`Account`](TokenAccount): a holder of
    /// `mint` owned by `owner` carrying `amount`, owned by `program`.
    #[allow(clippy::too_many_arguments)] // trait method; the token-account fields could form a spec struct
    fn fabricate_token_account(
        &mut self,
        account: &Pubkey,
        program: TokenProgram,
        mint: &Pubkey,
        owner: &Pubkey,
        amount: u64,
    );
}

impl TokenFabrication for LiteSVM {
    fn fabricate_mint(&mut self, mint: &Pubkey, program: TokenProgram, decimals: u8, supply: u64) {
        let m = Mint {
            mint_authority: COption::None,
            supply,
            decimals,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        let mut data = vec![0u8; Mint::LEN];
        Mint::pack(m, &mut data).expect("pack Mint");
        let rent = self.minimum_balance_for_rent_exemption(Mint::LEN);
        self.set_account(
            *mint,
            Account {
                lamports: rent,
                data,
                owner: program.id(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .expect("set fabricated mint");
    }

    fn fabricate_token_account(
        &mut self,
        account: &Pubkey,
        program: TokenProgram,
        mint: &Pubkey,
        owner: &Pubkey,
        amount: u64,
    ) {
        let acct = TokenAccount {
            mint: *mint,
            owner: *owner,
            amount,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };
        let mut data = vec![0u8; TokenAccount::LEN];
        TokenAccount::pack(acct, &mut data).expect("pack token account");
        let rent = self.minimum_balance_for_rent_exemption(TokenAccount::LEN);
        self.set_account(
            *account,
            Account {
                lamports: rent,
                data,
                owner: program.id(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .expect("set fabricated token account");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabricated_nft_mint_reads_as_an_nft() {
        let mut svm = LiteSVM::new();
        let mint = Pubkey::new_unique();
        svm.fabricate_nft_mint(&mint, TokenProgram::Spl);
        let acct = svm.get_account(&mint).expect("mint exists");
        assert_eq!(acct.owner, spl_token::id());
        let m = Mint::unpack(&acct.data).expect("unpacks as an SPL Mint");
        assert_eq!((m.supply, m.decimals, m.is_initialized), (1, 0, true));
    }

    #[test]
    fn fabricated_t22_mint_shares_layout_but_owner_differs() {
        let mut svm = LiteSVM::new();
        let mint = Pubkey::new_unique();
        svm.fabricate_nft_mint(&mint, TokenProgram::Token2022);
        let acct = svm.get_account(&mint).expect("mint exists");
        assert_eq!(acct.owner, TOKEN_2022_ID);
        // Same base layout, so the classic unpacker still reads it.
        let m = Mint::unpack(&acct.data).expect("base layout unpacks");
        assert_eq!((m.supply, m.decimals), (1, 0));
    }

    #[test]
    fn fabricated_token_account_carries_balance() {
        let mut svm = LiteSVM::new();
        let (account, mint, owner) = (
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        );
        svm.fabricate_token_account(&account, TokenProgram::Spl, &mint, &owner, 42);
        let acct = svm.get_account(&account).expect("token account exists");
        assert_eq!(acct.owner, spl_token::id());
        let ta = TokenAccount::unpack(&acct.data).expect("unpacks as a token account");
        assert_eq!((ta.mint, ta.owner, ta.amount), (mint, owner, 42));
    }
}
