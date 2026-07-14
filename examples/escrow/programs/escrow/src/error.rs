use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Escrow has expired")]
    EscrowExpired,
    #[msg("Escrow has not expired")]
    EscrowNotExpired,
}
