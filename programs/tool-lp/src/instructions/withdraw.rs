use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, Token2022, TokenAccount, TokenInterface, TransferChecked},
};
use raydium_cp_swap::{cpi, program::RaydiumCpSwap, states::PoolState};

use crate::{Error, UserLock, Vault, WithdrawEvent, ADMIN_WALLET};

pub fn handler(ctx: Context<Withdraw>, lp_token_amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let user_lock = &mut ctx.accounts.user_lock;
    let current_timestamp = Clock::get()?.unix_timestamp;

    let (token_0_amount, token_1_amount, fee_0_amount, fee_1_amount) = {
        let pool_state = ctx.accounts.pool_state.load()?;

        require!(
            ctx.accounts.token_0_vault.key() == pool_state.token_0_vault,
            Error::InvalidInput
        );
        require!(
            ctx.accounts.token_1_vault.key() == pool_state.token_1_vault,
            Error::InvalidInput
        );
        require!(
            ctx.accounts.lp_mint.key() == pool_state.lp_mint,
            Error::InvalidInput
        );
        require!(
            ctx.accounts.token_0_program.key() == pool_state.token_0_program,
            Error::InvalidInput
        );
        require!(
            ctx.accounts.token_1_program.key() == pool_state.token_1_program,
            Error::InvalidInput
        );

        require!(
            current_timestamp >= user_lock.unlock_timestamp,
            Error::LockNotYetExpired
        );

        require!(
            user_lock.amount >= lp_token_amount,
            Error::InsufficientBalance
        );

        let available_balance = ctx.accounts.vault_token_account.amount;
        require!(
            available_balance >= lp_token_amount,
            Error::InsufficientBalance
        );

        let (vault_0_amount, vault_1_amount) = pool_state.vault_amount_without_fee(
            ctx.accounts.token_0_vault.amount,
            ctx.accounts.token_1_vault.amount,
        );

        let raw_token_0_amount = lp_token_amount
            .checked_mul(vault_0_amount)
            .ok_or(Error::ArithmeticError)?
            .checked_div(pool_state.lp_supply)
            .ok_or(Error::ArithmeticError)?;

        let raw_token_1_amount = lp_token_amount
            .checked_mul(vault_1_amount)
            .ok_or(Error::ArithmeticError)?
            .checked_div(pool_state.lp_supply)
            .ok_or(Error::ArithmeticError)?;

        let deposit_token_0_amount = lp_token_amount
            .checked_mul(user_lock.deposit_token_per_lp_0)
            .ok_or(Error::ArithmeticError)?;

        let deposit_token_1_amount = lp_token_amount
            .checked_mul(user_lock.deposit_token_per_lp_1)
            .ok_or(Error::ArithmeticError)?;

        let growth_0 = (raw_token_0_amount as i128)
            .checked_sub(deposit_token_0_amount as i128)
            .ok_or(Error::ArithmeticError)?;

        let growth_1 = (raw_token_1_amount as i128)
            .checked_sub(deposit_token_1_amount as i128)
            .ok_or(Error::ArithmeticError)?;

        let fee_0_amount = if growth_0 > 0 {
            (growth_0 as u64)
                .checked_mul(20)
                .ok_or(Error::ArithmeticError)?
                .checked_div(100)
                .ok_or(Error::ArithmeticError)?
        } else {
            0
        };

        let fee_1_amount = if growth_1 > 0 {
            (growth_1 as u64)
                .checked_mul(20)
                .ok_or(Error::ArithmeticError)?
                .checked_div(100)
                .ok_or(Error::ArithmeticError)?
        } else {
            0
        };

        let token_0_amount = deposit_token_0_amount
            .checked_add(if growth_0 > 0 {
                (growth_0 as u64).checked_sub(fee_0_amount).unwrap_or(0)
            } else {
                0
            })
            .ok_or(Error::ArithmeticError)?;

        let token_1_amount = deposit_token_1_amount
            .checked_add(if growth_1 > 0 {
                (growth_1 as u64).checked_sub(fee_1_amount).unwrap_or(0)
            } else {
                0
            })
            .ok_or(Error::ArithmeticError)?;

        (token_0_amount, token_1_amount, fee_0_amount, fee_1_amount)
    };

    let vault_key = vault.key();
    let pool_state_key = ctx.accounts.pool_state.key();
    let seeds = &[
        b"vault-authority",
        pool_state_key.as_ref(),
        vault_key.as_ref(),
        &[vault.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = cpi::accounts::Withdraw {
        owner: ctx.accounts.vault_authority.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
        pool_state: ctx.accounts.pool_state.to_account_info(),
        owner_lp_token: ctx.accounts.vault_token_account.to_account_info(),
        token_0_account: ctx.accounts.vault_token_0_account.to_account_info(),
        token_1_account: ctx.accounts.vault_token_1_account.to_account_info(),
        token_0_vault: ctx.accounts.token_0_vault.to_account_info(),
        token_1_vault: ctx.accounts.token_1_vault.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        token_program_2022: ctx.accounts.token_program_2022.to_account_info(),
        vault_0_mint: ctx.accounts.vault_0_mint.to_account_info(),
        vault_1_mint: ctx.accounts.vault_1_mint.to_account_info(),
        lp_mint: ctx.accounts.lp_mint.to_account_info(),
        memo_program: ctx.accounts.memo_program.to_account_info(),
    };

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.cp_swap_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cpi::withdraw(cpi_context, lp_token_amount, token_0_amount, token_1_amount)?;

    if token_0_amount > 0 {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_0_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_0_account.to_account_info(),
                    to: ctx.accounts.user_token_0_account.to_account_info(),
                    mint: ctx.accounts.vault_0_mint.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            token_0_amount,
            ctx.accounts.vault_0_mint.decimals,
        )?;
    }

    if token_1_amount > 0 {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_1_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_1_account.to_account_info(),
                    to: ctx.accounts.user_token_1_account.to_account_info(),
                    mint: ctx.accounts.vault_1_mint.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            token_1_amount,
            ctx.accounts.vault_1_mint.decimals,
        )?;
    }

    if ctx
        .accounts
        .admin_token_0_account
        .to_account_info()
        .data_is_empty()
    {
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.user.to_account_info(),
                associated_token: ctx.accounts.admin_token_0_account.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
                mint: ctx.accounts.vault_0_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_0_program.to_account_info(),
            },
        ))?;
    }

    if ctx
        .accounts
        .admin_token_1_account
        .to_account_info()
        .data_is_empty()
    {
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.user.to_account_info(),
                associated_token: ctx.accounts.admin_token_1_account.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
                mint: ctx.accounts.vault_1_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_1_program.to_account_info(),
            },
        ))?;
    }

    if fee_0_amount > 0 {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_0_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_0_account.to_account_info(),
                    to: ctx.accounts.admin_token_0_account.to_account_info(),
                    mint: ctx.accounts.vault_0_mint.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            fee_0_amount,
            ctx.accounts.vault_0_mint.decimals,
        )?;
    }

    if fee_1_amount > 0 {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_1_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_1_account.to_account_info(),
                    to: ctx.accounts.admin_token_1_account.to_account_info(),
                    mint: ctx.accounts.vault_1_mint.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            fee_1_amount,
            ctx.accounts.vault_1_mint.decimals,
        )?;
    }

    user_lock.amount = user_lock
        .amount
        .checked_sub(lp_token_amount)
        .ok_or(Error::ArithmeticError)?;
    vault.total_locked = vault
        .total_locked
        .checked_sub(lp_token_amount)
        .ok_or(Error::ArithmeticError)?;

    emit!(WithdrawEvent {
        user: ctx.accounts.user.key(),
        vault: vault.key(),
        pool_state: vault.pool_state,
        lp_amount: lp_token_amount,
        token_0_amount,
        token_1_amount,
        fee_0_amount,
        fee_1_amount,
        timestamp: current_timestamp,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(lp_token_amount: u64)]
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
    #[account(mut, token::mint = token_mint, token::authority = vault_authority)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = vault_0_mint, token::authority = vault_authority)]
    pub vault_token_0_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut, token::mint = vault_1_mint, token::authority = vault_authority)]
    pub vault_token_1_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut, token::mint = vault_0_mint, token::authority = user)]
    pub user_token_0_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut, token::mint = vault_1_mint, token::authority = user)]
    pub user_token_1_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(address = ADMIN_WALLET)]
    /// CHECK: Hardcoded admin wallet from lib.rs
    pub admin: UncheckedAccount<'info>,
    /// CHECK: will be created if empty
    #[account(mut)]
    pub admin_token_0_account: UncheckedAccount<'info>,
    /// CHECK: will be created if empty
    #[account(mut)]
    pub admin_token_1_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub token_1_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [b"vault-authority", vault.pool_state.as_ref(), vault.key().as_ref()],
        bump = vault.bump
    )]
    /// CHECK: PDA verified via seeds
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut, address = vault.pool_state)]
    pub pool_state: AccountLoader<'info, PoolState>,
    pub cp_swap_program: Program<'info, RaydiumCpSwap>,
    #[account(  
        seeds = [  
            raydium_cp_swap::AUTH_SEED.as_bytes(),
        ],  
        seeds::program = cp_swap_program,  
        bump,  
    )]  
    /// CHECK: pool vault and lp mint authority
    pub authority: UncheckedAccount<'info>,
    pub token_mint: InterfaceAccount<'info, Mint>,

    pub vault_0_mint: Box<InterfaceAccount<'info, Mint>>,
    pub vault_1_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub lp_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_0_program: Interface<'info, TokenInterface>,
    pub token_1_program: Interface<'info, TokenInterface>,
    pub token_program: Interface<'info, TokenInterface>,
    pub token_program_2022: Program<'info, Token2022>,
    /// memo program
    /// CHECK:
    #[account(address = spl_memo::id())]
    pub memo_program: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}
