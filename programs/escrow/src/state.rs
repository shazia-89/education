use anchor_lang::prelude::*;

#[account]
pub struct EscrowState {
    pub sender: Pubkey,
    pub receiver: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub deal_id: u64,
    pub bump: u8,
    pub status: EscrowStatus,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum EscrowStatus {
    Created,
    Funded,
    Released,
    Cancelled,
}

impl EscrowState {
    pub const SPACE: usize = 8 + 32 + 32 + 32 + 8 + 8 + 1 + 1;
}
