pub fn calculate_fee(amount: u64, fee_basis_points: u64) -> u64 {
    amount * fee_basis_points / 10000
}
