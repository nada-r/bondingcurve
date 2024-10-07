use std::fmt;

/// Represents the result of a buy operation
#[derive(Debug)]
pub struct BuyResult {
    pub token_amount: u64,
    pub sol_amount: u64,
}

/// Represents the result of a sell operation
#[derive(Debug)]
pub struct SellResult {
    pub token_amount: u64,
    pub sol_amount: u64,
}

/// Represents the Automated Market Maker (AMM)
#[derive(Debug)]
pub struct AMM {
    pub virtual_sol_reserves: u128,
    pub virtual_token_reserves: u128,
    pub real_sol_reserves: u128,
    pub real_token_reserves: u128,
    pub initial_virtual_token_reserves: u128,
}

impl AMM {
    /// Creates a new AMM instance
    pub fn new(
        virtual_sol_reserves: u128,
        virtual_token_reserves: u128,
        real_sol_reserves: u128,
        real_token_reserves: u128,
        initial_virtual_token_reserves: u128,
    ) -> Self {
        AMM {
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves,
            initial_virtual_token_reserves,
        }
    }

    /// Calculates the buy price for a given amount of tokens
    pub fn get_buy_price(&self, tokens: u128) -> Option<u128> {
        // Return None if tokens is 0 or exceeds virtual token reserves
        if tokens == 0 || tokens > self.virtual_token_reserves {
            return None;
        }

        // Calculate the product of reserves
        let product_of_reserves = self
            .virtual_sol_reserves
            .checked_mul(self.virtual_token_reserves)?;

        // Calculate new virtual token reserves
        let new_virtual_token_reserves = self.virtual_token_reserves.checked_sub(tokens)?;

        // Calculate new virtual SOL reserves
        let new_virtual_sol_reserves = product_of_reserves
            .checked_div(new_virtual_token_reserves)?
            .checked_add(1)?;

        // Calculate the amount of SOL needed
        let amount_needed = new_virtual_sol_reserves.checked_sub(self.virtual_sol_reserves)?;

        Some(amount_needed)
    }

    /// Applies a buy operation to the AMM
    pub fn apply_buy(&mut self, token_amount: u128) -> Option<BuyResult> {
        // Determine the final token amount
        let final_token_amount = if token_amount > self.real_token_reserves {
            self.real_token_reserves
        } else {
            token_amount
        };

        // Get the SOL amount needed for the buy
        let sol_amount = self.get_buy_price(final_token_amount)?;

        // Update virtual and real token reserves
        self.virtual_token_reserves = self
            .virtual_token_reserves
            .checked_sub(final_token_amount)?;
        self.real_token_reserves = self.real_token_reserves.checked_sub(final_token_amount)?;

        // Update virtual and real SOL reserves
        self.virtual_sol_reserves = self.virtual_sol_reserves.checked_add(sol_amount)?;
        self.real_sol_reserves = self.real_sol_reserves.checked_add(sol_amount)?;

        Some(BuyResult {
            token_amount: final_token_amount as u64,
            sol_amount: sol_amount as u64,
        })
    }

    /// Applies a sell operation to the AMM
    pub fn apply_sell(&mut self, token_amount: u128) -> Option<SellResult> {
        // Update virtual and real token reserves
        self.virtual_token_reserves = self.virtual_token_reserves.checked_add(token_amount)?;
        self.real_token_reserves = self.real_token_reserves.checked_add(token_amount)?;

        // Get the SOL amount to be received from the sell
        let sol_amount = self.get_sell_price(token_amount)?;

        // Update virtual and real SOL reserves
        self.virtual_sol_reserves = self.virtual_sol_reserves.checked_sub(sol_amount)?;
        self.real_sol_reserves = self.real_sol_reserves.checked_sub(sol_amount)?;

        Some(SellResult {
            token_amount: token_amount as u64,
            sol_amount: sol_amount as u64,
        })
    }

    /// Calculates the sell price for a given amount of tokens
    pub fn get_sell_price(&self, tokens: u128) -> Option<u128> {
        // Return None if tokens is 0 or exceeds virtual token reserves
        if tokens == 0 || tokens > self.virtual_token_reserves {
            return None;
        }

        let scaling_factor = self.initial_virtual_token_reserves;

        // Scale the tokens
        let scaled_tokens = tokens.checked_mul(scaling_factor)?;

        // Calculate the token sell proportion
        let token_sell_proportion = scaled_tokens.checked_div(self.virtual_token_reserves)?;

        // Calculate the amount of SOL to be received
        let sol_received = (self
            .virtual_sol_reserves
            .checked_mul(token_sell_proportion)?)
        .checked_div(scaling_factor)?;

        // Return the minimum of calculated SOL and real SOL reserves
        Some(sol_received.min(self.real_sol_reserves))
    }
}

/// Implements the Display trait for AMM
impl fmt::Display for AMM {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AMM {{ virtual_sol_reserves: {}, virtual_token_reserves: {}, real_sol_reserves: {}, real_token_reserves: {}, initial_virtual_token_reserves: {} }}",
            self.virtual_sol_reserves,
            self.virtual_token_reserves,
            self.real_sol_reserves,
            self.real_token_reserves,
            self.initial_virtual_token_reserves
        )
    }
}
