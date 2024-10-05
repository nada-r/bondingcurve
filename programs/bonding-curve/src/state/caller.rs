use anchor_lang::prelude::*;

#[account]
pub struct Caller {
    pub caller: Pubkey,
    pub mint: Pubkey,
    pub mint_supply: u64,
    pub mint_total_supply: u64,
    pub value_target: u64,
    pub mint_vault: Pubkey,
    pub sol_vault_bump: u8,
    pub bump: u8,
}

impl Space for Caller {
    const INIT_SPACE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 32 + 1 + 1;
}
