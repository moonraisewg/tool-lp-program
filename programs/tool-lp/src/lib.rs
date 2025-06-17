use anchor_lang::prelude::*;

pub mod instructions;
use instructions::*;

declare_id!("DduTe3VFPwWGN2EBh8FZ1GSnXe7VFotp1A8eej7qwgX2");

pub const ADMIN_WALLET: Pubkey = pubkey!("As1T4LoB97vriM5HWXy2Z23s8Sp9ymZgibnnc2r9mCQZ");

#[program]
pub mod tool_lp {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize_vault::handler(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, unlock_timestamp: i64) -> Result<()> {
        instructions::deposit::handler(ctx, amount, unlock_timestamp)
    }

    pub fn withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, lp_token_amount)
    }
}

#[account]
pub struct Vault {
    pub pool_state: Pubkey,
    pub token_mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub total_locked: u64,
    pub bump: u8,
}

impl Vault {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 1;
}

#[account]
pub struct UserLock {
    pub user: Pubkey,
    pub amount: u64,
    pub unlock_timestamp: i64,
    pub deposit_token_per_lp_0: u64,
    pub deposit_token_per_lp_1: u64,
}

impl UserLock {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 8;
}

#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub pool_state: Pubkey,
    pub amount: u64,
    pub unlock_timestamp: i64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub pool_state: Pubkey,
    pub lp_amount: u64,
    pub token_0_amount: u64,
    pub token_1_amount: u64,
    pub fee_0_amount: u64,
    pub fee_1_amount: u64,
    pub timestamp: i64,
}

#[error_code]
pub enum Error {
    #[msg("Lock period has not yet expired")]
    LockNotYetExpired,
    #[msg("Invalid input: mint, vault, program, or timestamp")]
    InvalidInput,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Arithmetic calculation error")]
    ArithmeticError,
    #[msg("Account not initialized")]
    AccountNotInitialized,
}
