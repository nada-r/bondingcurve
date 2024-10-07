use anchor_lang::prelude::*;

use contexts::*;

pub mod amm;
pub mod constants;
pub mod contexts;
pub mod errors;
pub mod state;
pub mod util;

declare_id!("HCrubXr4ZpBewADnw7nDFmYixvqcwYqz3CxXHPqhHuTn");

#[program]
pub mod bonding_curve {

    use super::*;

    /// Initializes the bonding curve with the given parameters
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
        initialize::initialize(
            ctx,
            fee_recipient,
            withdraw_authority,
            initial_virtual_token_reserves,
            initial_virtual_sol_reserves,
            initial_real_token_reserves,
            initial_token_supply,
            fee_basis_points,
        )
    }

    /// Creates a new token with the given name, symbol, and URI
    pub fn create(ctx: Context<Create>, name: String, symbol: String, uri: String) -> Result<()> {
        create::create(ctx, name, symbol, uri)
    }

    /// Buys tokens from the bonding curve
    pub fn buy(ctx: Context<Buy>, token_amount: u64, max_sol_cost: u64) -> Result<()> {
        buy::buy(ctx, token_amount, max_sol_cost)
    }

    /// Sells tokens back to the bonding curve
    pub fn sell(ctx: Context<Sell>, token_amount: u64, min_sol_output: u64) -> Result<()> {
        sell::sell(ctx, token_amount, min_sol_output)
    }

    /// Withdraws accumulated fees
    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        withdraw::withdraw(ctx)
    }
}
