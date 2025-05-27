use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use raydium_cp_swap::states::PoolState;

use crate::{DepositEvent, UserLock, Vault, VaultError};

pub fn handler(ctx: Context<Deposit>, amount: u64, unlock_timestamp: i64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let user_lock = &mut ctx.accounts.user_lock;
    let current_timestamp = Clock::get()?.unix_timestamp;

    require!(
        unlock_timestamp > current_timestamp,
        VaultError::InvalidUnlockTimestamp
    );

    require!(
        ctx.accounts.user_token_account.mint == vault.token_mint,
        VaultError::InvalidMint
    );

    let pool_state = ctx.accounts.pool_state.load()?;
    require!(
        pool_state.lp_mint == vault.token_mint,
        VaultError::InvalidMint
    );

    let (vault_0_amount, vault_1_amount) = pool_state.vault_amount_without_fee(
        ctx.accounts.token_0_vault.amount,
        ctx.accounts.token_1_vault.amount,
    );
    let deposit_token_per_lp_0 = vault_0_amount
        .checked_div(pool_state.lp_supply)
        .ok_or(VaultError::ArithmeticUnderflow)?;
    let deposit_token_per_lp_1 = vault_1_amount
        .checked_div(pool_state.lp_supply)
        .ok_or(VaultError::ArithmeticUnderflow)?;

    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.token_mint.decimals,
    )?;

    user_lock.user = ctx.accounts.user.key();
    user_lock.amount = user_lock
        .amount
        .checked_add(amount)
        .ok_or(VaultError::ArithmeticOverflow)?;
    user_lock.unlock_timestamp = unlock_timestamp;
    user_lock.deposit_token_per_lp_0 = deposit_token_per_lp_0;
    user_lock.deposit_token_per_lp_1 = deposit_token_per_lp_1;

    vault.total_locked = vault
        .total_locked
        .checked_add(amount)
        .ok_or(VaultError::ArithmeticOverflow)?;

    emit!(DepositEvent {
        user: ctx.accounts.user.key(),
        vault: vault.key(),
        pool_state: vault.pool_state,
        amount,
        unlock_timestamp,
        timestamp: current_timestamp,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, unlock_timestamp: i64)]
pub struct Deposit<'info> {
    #[account(mut)]
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
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = token_mint)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(address = vault.pool_state)]
    pub pool_state: AccountLoader<'info, PoolState>,
    #[account(
        mut,
        constraint = token_0_vault.key() == pool_state.load()?.token_0_vault
    )]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = token_1_vault.key() == pool_state.load()?.token_1_vault
    )]
    pub token_1_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>,
}
