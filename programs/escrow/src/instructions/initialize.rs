use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{error::EscrowError, state::{EscrowState, EscrowStatus}};

#[derive(Accounts)]
#[instruction(deal_id: u64, amount: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    pub receiver: SystemAccount<'info>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = sender,
        space = EscrowState::SPACE,
        seeds = [b"escrow", sender.key().as_ref(), deal_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub state: Account<'info, EscrowState>,
    #[account(
        init,
        payer = sender,
        associated_token::mint = mint,
        associated_token::authority = state,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, deal_id: u64, amount: u64) -> Result<()> {
    require!(amount > 0, EscrowError::AmountMustBePositive);
    require!(ctx.accounts.sender.key() != ctx.accounts.receiver.key(), EscrowError::SameParty);

    let state = &mut ctx.accounts.state;
    state.sender = ctx.accounts.sender.key();
    state.receiver = ctx.accounts.receiver.key();
    state.mint = ctx.accounts.mint.key();
    state.amount = amount;
    state.deal_id = deal_id;
    state.bump = ctx.bumps.state;
    state.status = EscrowStatus::Created;
    Ok(())
}
