use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{error::EscrowError, state::{EscrowState, EscrowStatus}};

#[derive(Accounts)]
pub struct Release<'info> {
    pub sender: Signer<'info>,
    #[account(
        mut,
        close = sender,
        has_one = sender,
        constraint = state.status == EscrowStatus::Funded @ EscrowError::InvalidState,
        constraint = state.mint == mint.key() @ EscrowError::DealIdMismatch,
    )]
    pub state: Account<'info, EscrowState>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = receiver,
        token::token_program = token_program,
    )]
    pub receiver_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: constrained to the receiver stored in state.
    #[account(address = state.receiver)]
    pub receiver: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Release>) -> Result<()> {
    let amount = ctx.accounts.state.amount;
    let decimals = ctx.accounts.mint.decimals;
    let deal_id_bytes = ctx.accounts.state.deal_id.to_le_bytes();
    let bump_bytes = [ctx.accounts.state.bump];
    let seeds: &[&[u8]] = &[
        b"escrow",
        ctx.accounts.state.sender.as_ref(),
        &deal_id_bytes,
        &bump_bytes,
    ];
    let signer = &[seeds];
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.receiver_token_account.to_account_info(),
        authority: ctx.accounts.state.to_account_info(),
    };
    let cpi = CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer);
    token_interface::transfer_checked(cpi, amount, decimals)?;
    let close_accounts = token_interface::CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.sender.to_account_info(),
        authority: ctx.accounts.state.to_account_info(),
    };
    let close_cpi = CpiContext::new_with_signer(ctx.accounts.token_program.key(), close_accounts, signer);
    token_interface::close_account(close_cpi)?;
    Ok(())
}
