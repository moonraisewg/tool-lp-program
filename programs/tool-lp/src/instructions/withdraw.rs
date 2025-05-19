use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};

use crate::{WithdrawEvent, UserLock, Vault, VaultError};

pub fn handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let user_lock = &mut ctx.accounts.user_lock;
    let current_timestamp = Clock::get()?.unix_timestamp;

    // Check if lock period has ended
    require!(
        current_timestamp >= user_lock.unlock_timestamp,
        VaultError::LockNotYetExpired
    );

    // Check if user has enough locked tokens
    require!(user_lock.amount >= amount, VaultError::InsufficientBalance);

    // Check if vault has enough tokens
    let available_balance = ctx.accounts.vault_token_account.amount;
    require!(
        available_balance >= amount,
        VaultError::InsufficientBalance
    );

    // Calculate PDA seeds for vault authority
    let vault_key = vault.key();
    let seeds = &[
        b"vault-authority",
        vault.pool_id.as_ref(),
        vault_key.as_ref(),
        &[vault.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Transfer tokens from vault to user
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    // Update user locked amount
    user_lock.amount = user_lock
        .amount
        .checked_sub(amount)
        .ok_or(VaultError::ArithmeticUnderflow)?;

    // Update total locked tokens in vault
    vault.total_locked = vault
        .total_locked
        .checked_sub(amount)
        .ok_or(VaultError::ArithmeticUnderflow)?;

    // Emit withdraw event
    emit!(WithdrawEvent {
        user: ctx.accounts.user.key(),
        vault: vault.key(),
        pool_id: vault.pool_id,
        amount,
        timestamp: current_timestamp,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = token_mint)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user-lock", vault.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_lock: Account<'info, UserLock>,
    #[account(mut, token::mint = token_mint)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut, token::mint = token_mint)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"vault-authority", vault.pool_id.as_ref(), vault.key().as_ref()],
        bump = vault.bump
    )]
    /// CHECK: PDA verified via seeds
    pub vault_authority: UncheckedAccount<'info>,
    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}