use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{Vault, VaultError};

pub fn handler(ctx: Context<InitializeVault>, pool_id: Pubkey, bump: u8) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.pool_id = pool_id;
    vault.token_mint = ctx.accounts.token_mint.key();
    vault.vault_token_account = ctx.accounts.vault_token_account.key();
    vault.total_locked = 0;
    vault.bump = bump;

    Ok(())
}

#[derive(Accounts)]
#[instruction(pool_id: Pubkey, bump: u8)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = initializer,
        space = 8 + Vault::LEN,
        seeds = [b"vault", pool_id.as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub token_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = initializer,
        token::mint = token_mint,
        token::authority = vault_authority,
        seeds = [b"vault-token", pool_id.as_ref(), vault.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"vault-authority", pool_id.as_ref(), vault.key().as_ref()],
        bump
    )]
    /// CHECK: PDA verified via seeds
    pub vault_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}