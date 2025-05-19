use anchor_lang::prelude::*;

pub mod instructions;
use instructions::*;

declare_id!("Hog1fQ9MwCd6qQFoVYczbbXwEWNd3m1bnNakPGg4frK");

#[program]
pub mod tool_lp {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>, pool_id: Pubkey, bump: u8) -> Result<()> {
        instructions::initialize_vault::handler(ctx, pool_id, bump)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        unlock_timestamp: i64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, amount, unlock_timestamp)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, amount)
    }

    pub fn proxy_withdraw(
        ctx: Context<ProxyWithdraw>,
        lp_token_amount: u64,
        minimum_token_0_amount: u64,
        minimum_token_1_amount: u64,
    ) -> Result<()> {
        instructions::proxy_withdraw(
            ctx,
            lp_token_amount,
            minimum_token_0_amount,
            minimum_token_1_amount,
        )
    }
}

#[account]
pub struct Vault {
    pub pool_id: Pubkey,            // Raydium pool ID
    pub token_mint: Pubkey,         // LP token mint
    pub vault_token_account: Pubkey,// Vault token account
    pub total_locked: u64,          // Total locked tokens
    pub bump: u8,                   // Bump seed for PDA
}

impl Vault {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 1; // Pubkeys + u64 + u8
}

#[account]
pub struct UserLock {
    pub user: Pubkey,         // User address
    pub amount: u64,          // Locked token amount
    pub unlock_timestamp: i64, // Unlock timestamp
}

impl UserLock {
    pub const LEN: usize = 32 + 8 + 8; // Pubkey + u64 + i64
}

#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub pool_id: Pubkey,
    pub amount: u64,
    pub unlock_timestamp: i64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub pool_id: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[error_code]
pub enum VaultError {
    #[msg("Lock period has not yet expired")]
    LockNotYetExpired,
    #[msg("Invalid LP token mint")]
    InvalidMint,
    #[msg("Insufficient balance to withdraw")]
    InsufficientBalance,
    #[msg("Arithmetic overflow error")]
    ArithmeticOverflow,
    #[msg("Arithmetic underflow error")]
    ArithmeticUnderflow,
    #[msg("Invalid unlock timestamp")]
    InvalidUnlockTimestamp,
}
