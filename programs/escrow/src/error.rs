use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    AmountMustBePositive,
    #[msg("Sender and receiver must be different")]
    SameParty,
    #[msg("Invalid escrow state for this operation")]
    InvalidState,
    #[msg("Deal id does not match escrow state")]
    DealIdMismatch,
}
