use anchor_lang::{prelude::*, solana_program::system_instruction};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{
    amm,
    errors::BondingCurveError,
    state::{BondingCurve, Config},
    util::calculate_fee,
};

#[derive(Accounts)]
pub struct Buy<'info> {
    /// The user initiating the buy transaction
    #[account(mut)]
    user: Signer<'info>,

    /// The global configuration account
    #[account(
        seeds = [Config::SEED_PREFIX],
        bump,
    )]
    config: Box<Account<'info, Config>>,

    /// The account that will receive the fee
    /// CHECK: Using config state to validate fee_recipient account
    #[account(mut)]
    fee_recipient: AccountInfo<'info>,

    /// The mint of the token being bought
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

    /// The user's token account to receive the bought tokens
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<Account<'info, TokenAccount>>,

    /// The System program
    system_program: Program<'info, System>,

    /// The Token program
    token_program: Program<'info, Token>,
}

/// Buy instruction handler
pub fn buy(ctx: Context<Buy>, token_amount: u64, max_sol_cost: u64) -> Result<()> {
    // Ensure the program is initialized
    require!(
        ctx.accounts.config.initialized,
        BondingCurveError::NotInitialized
    );

    // Ensure the bonding curve is not complete
    require!(
        !ctx.accounts.bonding_curve.complete,
        BondingCurveError::BondingCurveComplete,
    );

    // Validate the fee recipient
    require!(
        ctx.accounts.fee_recipient.key == &ctx.accounts.config.fee_recipient,
        BondingCurveError::InvalidFeeRecipient,
    );

    // Ensure the bonding curve has enough tokens
    require!(
        ctx.accounts.bonding_curve.real_token_reserves >= token_amount,
        BondingCurveError::InsufficientTokens,
    );

    // Ensure the token amount is greater than zero
    require!(token_amount > 0, BondingCurveError::MinBuy,);

    // Calculate the actual token amount to buy
    let targe_token_amount = if ctx.accounts.bonding_curve_token_account.amount < token_amount {
        ctx.accounts.bonding_curve_token_account.amount
    } else {
        token_amount
    };

    // Initialize the AMM with current state
    let mut amm = amm::amm::AMM::new(
        ctx.accounts.bonding_curve.virtual_sol_reserves as u128,
        ctx.accounts.bonding_curve.virtual_token_reserves as u128,
        ctx.accounts.bonding_curve.real_sol_reserves as u128,
        ctx.accounts.bonding_curve.real_token_reserves as u128,
        ctx.accounts.config.initial_virtual_token_reserves as u128,
    );

    // Apply the buy to the AMM
    let buy_result = amm.apply_buy(targe_token_amount as u128).unwrap();
    let fee = calculate_fee(buy_result.sol_amount, ctx.accounts.config.fee_basis_points);
    let buy_amount_with_fee = buy_result.sol_amount + fee;

    // Check if the amount of SOL to transfer plus fee is less than the max_sol_cost
    require!(
        buy_amount_with_fee <= max_sol_cost,
        BondingCurveError::MaxSOLCostExceeded,
    );

    // Check if the user has enough SOL
    require!(
        ctx.accounts.user.lamports() >= buy_amount_with_fee,
        BondingCurveError::InsufficientSOL,
    );

    // Transfer SOL to bonding curve
    let from_account = &ctx.accounts.user;
    let to_bonding_curve_account = &ctx.accounts.bonding_curve;

    let transfer_instruction = system_instruction::transfer(
        from_account.key,
        to_bonding_curve_account.to_account_info().key,
        buy_result.sol_amount,
    );

    anchor_lang::solana_program::program::invoke_signed(
        &transfer_instruction,
        &[
            from_account.to_account_info(),
            to_bonding_curve_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[],
    )?;

    // Transfer SOL to fee recipient
    let to_fee_recipient_account = &ctx.accounts.fee_recipient;

    let transfer_instruction =
        system_instruction::transfer(from_account.key, to_fee_recipient_account.key, fee);

    anchor_lang::solana_program::program::invoke_signed(
        &transfer_instruction,
        &[
            from_account.to_account_info(),
            to_fee_recipient_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[],
    )?;

    // Transfer SPL tokens
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
        buy_result.token_amount,
    )?;

    // Apply the buy to the bonding curve
    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.real_token_reserves = amm.real_token_reserves as u64;
    bonding_curve.real_sol_reserves = amm.real_sol_reserves as u64;
    bonding_curve.virtual_token_reserves = amm.virtual_token_reserves as u64;
    bonding_curve.virtual_sol_reserves = amm.virtual_sol_reserves as u64;

    // Check if the bonding curve is complete
    if bonding_curve.real_token_reserves == 0 {
        bonding_curve.complete = true;
    }

    Ok(())
}
