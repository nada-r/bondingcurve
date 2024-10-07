use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{
    errors::BondingCurveError,
    state::{BondingCurve, Config},
};

/// Accounts struct for the withdraw instruction
#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// The user initiating the withdrawal (must be the withdraw authority)
    #[account(mut)]
    user: Signer<'info>,

    /// The global configuration account
    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
    )]
    config: Box<Account<'info, Config>>,

    /// The mint of the token being withdrawn
    mint: Account<'info, Mint>,

    /// The bonding curve account associated with this mint
    #[account(
        mut,
        seeds = [BondingCurve::SEED_PREFIX, mint.to_account_info().key.as_ref()],
        bump,
    )]
    bonding_curve: Box<Account<'info, BondingCurve>>,

    /// The token account holding the tokens for the bonding curve
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    bonding_curve_token_account: Box<Account<'info, TokenAccount>>,

    /// The user's token account to receive the withdrawn tokens
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<Account<'info, TokenAccount>>,

    /// The Associated Token program
    associated_token_program: Program<'info, AssociatedToken>,

    /// The System program
    system_program: Program<'info, System>,

    /// The Token program
    token_program: Program<'info, Token>,
}

/// Withdraw instruction handler
pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
    // Ensure the program is initialized
    require!(
        ctx.accounts.config.initialized,
        BondingCurveError::NotInitialized
    );

    // Ensure the bonding curve is complete
    require!(
        ctx.accounts.bonding_curve.complete,
        BondingCurveError::BondingCurveNotComplete,
    );

    // Ensure the user is the withdraw authority
    require!(
        ctx.accounts.user.key() == ctx.accounts.config.withdraw_authority,
        BondingCurveError::InvalidWithdrawAuthority,
    );

    // Transfer tokens from bonding curve to withdraw authority
    let cpi_accounts = Transfer {
        from: ctx
            .accounts
            .bonding_curve_token_account
            .to_account_info()
            .clone(),
        to: ctx.accounts.user_token_account.to_account_info().clone(),
        authority: ctx.accounts.bonding_curve.to_account_info().clone(),
    };

    let signer: [&[&[u8]]; 1] = [&[
        BondingCurve::SEED_PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[ctx.bumps.bonding_curve],
    ]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &signer,
        ),
        ctx.accounts.bonding_curve_token_account.amount,
    )?;

    // Transfer SOL from bonding curve to withdraw authority
    let from_account = &ctx.accounts.bonding_curve;
    let to_account = &ctx.accounts.user;

    // Calculate the minimum balance required for rent exemption
    let min_balance = Rent::get()?.minimum_balance(8 + BondingCurve::INIT_SPACE);

    // Calculate the total amount of lamports to transfer
    let total_bonding_curve_lamports = from_account.get_lamports() - min_balance;

    // Perform the SOL transfer
    **from_account.to_account_info().try_borrow_mut_lamports()? -= total_bonding_curve_lamports;
    **to_account.try_borrow_mut_lamports()? += total_bonding_curve_lamports;

    Ok(())
}
