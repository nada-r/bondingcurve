use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub withdraw_authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub initial_real_sol_reserves: u64,
    pub initial_token_supply: u64,
    pub initialized: bool,
}

impl Config {
    pub const SEED_PREFIX: &'static [u8; 6] = b"config";
}
