use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};


use crate::{DepositEvent, UserLock, Vault, VaultError};

pub fn handler(
    ctx: Context<Deposit>,
    amount: u64,
    unlock_timestamp: i64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let user_lock = &mut ctx.accounts.user_lock;
    let current_timestamp = Clock::get()?.unix_timestamp;

    // Validate unlock timestamp is in the future
    require!(
        unlock_timestamp > current_timestamp,
        VaultError::InvalidUnlockTimestamp
    );

    // Validate user token account mint matches vault token mint
    require!(
        ctx.accounts.user_token_account.mint == vault.token_mint,
        VaultError::InvalidMint
    );

    // Transfer LP tokens from user to vault
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    // Update user lock information
    user_lock.user = ctx.accounts.user.key();
    user_lock.amount = user_lock
        .amount
        .checked_add(amount)
        .ok_or(VaultError::ArithmeticOverflow)?;
    user_lock.unlock_timestamp = unlock_timestamp;

    // Update total locked tokens in vault
    vault.total_locked = vault
        .total_locked
        .checked_add(amount)
        .ok_or(VaultError::ArithmeticOverflow)?;

    // Emit deposit event
    emit!(DepositEvent {
        user: ctx.accounts.user.key(),
        vault: vault.key(),
        pool_id: vault.pool_id,
        amount,
        unlock_timestamp,
        timestamp: current_timestamp,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, unlock_timestamp: i64)]
pub struct Deposit<'info> {
    #[account(mut, has_one = token_mint)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserLock::LEN,
        seeds = [b"user-lock", vault.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_lock: Account<'info, UserLock>,
    #[account(mut, token::mint = token_mint)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut, token::mint = token_mint)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>,
}