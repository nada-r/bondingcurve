// Import necessary modules and types
use crate::{
    amm,
    errors::BondingCurveError,
    state::{BondingCurve, Config},
    util::calculate_fee,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// Define the Sell context structure
#[derive(Accounts)]
pub struct Sell<'info> {
    // User account (signer)
    #[account(mut)]
    user: Signer<'info>,

    // Global configuration account
    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
    )]
    config: Box<Account<'info, Config>>,

    // Fee recipient account
    /// CHECK: Using config state to validate fee_recipient account
    #[account(mut)]
    fee_recipient: AccountInfo<'info>,

    // Token mint account
    mint: Account<'info, Mint>,

    // Bonding curve account
    #[account(
        mut,
        seeds = [BondingCurve::SEED_PREFIX, mint.to_account_info().key.as_ref()],
        bump,
    )]
    bonding_curve: Box<Account<'info, BondingCurve>>,

    // Bonding curve's token account
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    bonding_curve_token_account: Box<Account<'info, TokenAccount>>,

    // User's token account
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<Account<'info, TokenAccount>>,

    // System program
    system_program: Program<'info, System>,

    // Token program
    token_program: Program<'info, Token>,
}

// Sell instruction handler
pub fn sell(ctx: Context<Sell>, token_amount: u64, min_sol_output: u64) -> Result<()> {
    // Check if bonding curve is complete
    require!(
        !ctx.accounts.bonding_curve.complete,
        BondingCurveError::BondingCurveComplete,
    );

    // Confirm user has enough tokens
    require!(
        ctx.accounts.user_token_account.amount >= token_amount,
        BondingCurveError::InsufficientTokens,
    );

    // Validate fee recipient
    require!(
        ctx.accounts.fee_recipient.key == &ctx.accounts.config.fee_recipient,
        BondingCurveError::InvalidFeeRecipient,
    );

    // Confirm bonding curve has enough tokens
    require!(
        ctx.accounts.bonding_curve_token_account.amount >= token_amount,
        BondingCurveError::InsufficientTokens,
    );

    // Ensure token amount is greater than zero
    require!(token_amount > 0, BondingCurveError::MinSell,);

    // Initialize AMM with current state
    let mut amm = amm::amm::AMM::new(
        ctx.accounts.bonding_curve.virtual_sol_reserves as u128,
        ctx.accounts.bonding_curve.virtual_token_reserves as u128,
        ctx.accounts.bonding_curve.real_sol_reserves as u128,
        ctx.accounts.bonding_curve.real_token_reserves as u128,
        ctx.accounts.config.initial_virtual_token_reserves as u128,
    );

    // Apply sell operation to AMM
    let sell_result = amm.apply_sell(token_amount as u128).unwrap();
    let fee = calculate_fee(sell_result.sol_amount, ctx.accounts.config.fee_basis_points);

    // Calculate SOL amount after fee deduction
    let sell_amount_minus_fee = sell_result.sol_amount - fee;

    // Confirm minimum SOL output is met
    require!(
        sell_amount_minus_fee >= min_sol_output,
        BondingCurveError::MinSOLOutputExceeded,
    );

    // Transfer SPL tokens from user to bonding curve
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info().clone(),
        to: ctx
            .accounts
            .bonding_curve_token_account
            .to_account_info()
            .clone(),
        authority: ctx.accounts.user.to_account_info().clone(),
    };

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &[],
        ),
        sell_result.token_amount,
    )?;

    // Transfer SOL from bonding curve to user
    let from_account = &ctx.accounts.bonding_curve;
    let to_account = &ctx.accounts.user;

    **from_account.to_account_info().try_borrow_mut_lamports()? -= sell_result.sol_amount;
    **to_account.try_borrow_mut_lamports()? += sell_result.sol_amount;

    // Transfer fee to fee recipient
    **from_account.to_account_info().try_borrow_mut_lamports()? -= fee;
    **ctx.accounts.fee_recipient.try_borrow_mut_lamports()? += fee;

    // Update bonding curve state
    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.real_token_reserves = amm.real_token_reserves as u64;
    bonding_curve.real_sol_reserves = amm.real_sol_reserves as u64;
    bonding_curve.virtual_token_reserves = amm.virtual_token_reserves as u64;
    bonding_curve.virtual_sol_reserves = amm.virtual_sol_reserves as u64;

    Ok(())
}
