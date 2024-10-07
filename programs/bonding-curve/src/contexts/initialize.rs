use crate::{errors::BondingCurveError, state::Config};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    authority: Signer<'info>,

    #[account(
        init,
        space = 8 + Config::INIT_SPACE,
        seeds = [Config::SEED_PREFIX],
        bump,
        payer = authority,
    )]
    config: Box<Account<'info, Config>>,

    system_program: Program<'info, System>,
}

pub fn initialize(
    ctx: Context<Initialize>,
    fee_recipient: Pubkey,
    withdraw_authority: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    initial_token_supply: u64,
    fee_basis_points: u64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    require!(!config.initialized, BondingCurveError::AlreadyInitialized,);

    config.authority = *ctx.accounts.authority.to_account_info().key;
    config.withdraw_authority = withdraw_authority;

    config.fee_basis_points = fee_basis_points;
    config.fee_recipient = fee_recipient;

    config.initial_real_sol_reserves = 0;
    config.initial_token_supply = initial_token_supply;
    config.initial_real_token_reserves = initial_real_token_reserves;
    config.initial_virtual_sol_reserves = initial_virtual_sol_reserves;
    config.initial_virtual_token_reserves = initial_virtual_token_reserves;
    config.initialized = true;

    msg!("Initialized config state");

    Ok(())
}
