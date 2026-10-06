use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{error::EscrowError, state::{EscrowState, EscrowStatus}};

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub sender: Signer<'info>,
    #[account(
        mut,
        has_one = sender,
        constraint = state.status == EscrowStatus::Created @ EscrowError::InvalidState,
        constraint = state.mint == mint.key() @ EscrowError::DealIdMismatch,
    )]
    pub state: Account<'info, EscrowState>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = sender,
        token::token_program = token_program,
    )]
    pub source: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = state,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Deposit>) -> Result<()> {
    let amount = ctx.accounts.state.amount;
    let decimals = ctx.accounts.mint.decimals;
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.source.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.sender.to_account_info(),
    };
    let cpi = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
    token_interface::transfer_checked(cpi, amount, decimals)?;
    ctx.accounts.state.status = EscrowStatus::Funded;
    Ok(())
}
