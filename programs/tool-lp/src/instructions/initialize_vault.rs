use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use raydium_cp_swap::states::PoolState;

use crate::{Error, Vault};

pub fn handler(ctx: Context<InitializeVault>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let pool_state = ctx.accounts.pool_state.load()?;

    require!(
        pool_state.lp_mint == ctx.accounts.token_mint.key(),
        Error::InvalidInput
    );

    require!(
        ctx.accounts.token_0_vault.key() == pool_state.token_0_vault,
        Error::InvalidInput
    );
    require!(
        ctx.accounts.token_1_vault.key() == pool_state.token_1_vault,
        Error::InvalidInput
    );

    vault.pool_state = ctx.accounts.pool_state.key();
    vault.token_mint = ctx.accounts.token_mint.key();
    vault.vault_token_account = ctx.accounts.vault_token_account.key();
    vault.total_locked = 0;
    vault.bump = ctx.bumps.vault;

    if ctx
        .accounts
        .vault_token_0_account
        .to_account_info()
        .data_is_empty()
    {
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.initializer.to_account_info(),
                associated_token: ctx.accounts.vault_token_0_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
                mint: ctx.accounts.vault_0_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_0_program.to_account_info(),
            },
        ))?;
    }

    if ctx
        .accounts
        .vault_token_1_account
        .to_account_info()
        .data_is_empty()
    {
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.initializer.to_account_info(),
                associated_token: ctx.accounts.vault_token_1_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
                mint: ctx.accounts.vault_1_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_1_program.to_account_info(),
            },
        ))?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = initializer,
        space = 8 + Vault::LEN,
        seeds = [b"vault", pool_state.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(address = pool_state.load()?.token_0_vault)]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(address = pool_state.load()?.token_1_vault)]
    pub token_1_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(address = token_0_vault.mint)]
    pub vault_0_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(address = token_1_vault.mint)]
    pub vault_1_mint: Box<InterfaceAccount<'info, Mint>>,
    pub pool_state: AccountLoader<'info, PoolState>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = initializer,
        token::mint = token_mint,
        token::authority = vault_authority,
        seeds = [b"vault-token", pool_state.key().as_ref(), vault.key().as_ref()],
        bump
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Will be created if empty
    #[account(mut)]
    pub vault_token_0_account: UncheckedAccount<'info>,
    /// CHECK: Will be created if empty
    #[account(mut)]
    pub vault_token_1_account: UncheckedAccount<'info>,
    #[account(
        seeds = [b"vault-authority", pool_state.key().as_ref(), vault.key().as_ref()],
        bump
    )]
    /// CHECK: PDA verified via seeds
    pub vault_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub token_0_program: Interface<'info, TokenInterface>,
    pub token_1_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
