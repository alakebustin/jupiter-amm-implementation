/// Bonding curve math — constant product with virtual reserves.
///
/// Calculate fee: fee = (amount * fee_bps) / 10_000
pub fn calculate_fee(amount: u64, fee_bps: u16) -> Option<u64> {
    let fee = (amount as u128)
        .checked_mul(fee_bps as u128)?
        .checked_div(10_000)?;
    Some(fee as u64)
}

/// Base creator fee rate in bps (0.3%)
const BASE_CREATOR_FEE_BPS: u32 = 30;

/// Compute effective creator fee bps from per-token multiplier.
/// effective = (30 * creator_fee_bps) / 10000
pub fn effective_creator_fee_bps(creator_fee_bps: u16) -> u16 {
    ((BASE_CREATOR_FEE_BPS * creator_fee_bps as u32) / 10000) as u16
}

/// Buy (ExactIn): given SOL input, return tokens out.
/// Fees deducted BEFORE swap math.
///
/// Returns (tokens_out, total_fee) or None on overflow.
pub fn calculate_buy(
    sol_amount: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_token_reserves: u64,
    protocol_fee_bps: u16,
    creator_fee_bps: u16, // per-token multiplier 0-10000
) -> Option<(u64, u64)> {
    let eff_creator_bps = effective_creator_fee_bps(creator_fee_bps);
    let protocol_fee = calculate_fee(sol_amount, protocol_fee_bps)?;
    let creator_fee = calculate_fee(sol_amount, eff_creator_bps)?;
    let total_fee = protocol_fee.checked_add(creator_fee)?;
    let sol_after_fee = sol_amount.checked_sub(total_fee)?;

    let k = (virtual_sol_reserves as u128).checked_mul(virtual_token_reserves as u128)?;
    let new_virtual_sol = (virtual_sol_reserves as u128).checked_add(sol_after_fee as u128)?;
    let new_virtual_tokens = k.checked_div(new_virtual_sol)?;
    let tokens_out = (virtual_token_reserves as u128).checked_sub(new_virtual_tokens)?;
    let tokens_out = tokens_out.min(real_token_reserves as u128);

    Some((tokens_out as u64, total_fee))
}

/// Sell (ExactIn): given token input, return SOL out.
/// Swap math BEFORE fees (fees deducted from output).
///
/// Returns (sol_out, total_fee) or None on overflow.
pub fn calculate_sell(
    token_amount: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_sol_reserves: u64,
    protocol_fee_bps: u16,
    creator_fee_bps: u16, // per-token multiplier 0-10000
) -> Option<(u64, u64)> {
    let k = (virtual_sol_reserves as u128).checked_mul(virtual_token_reserves as u128)?;
    let new_virtual_tokens = (virtual_token_reserves as u128).checked_add(token_amount as u128)?;
    let new_virtual_sol = k.checked_div(new_virtual_tokens)?;
    let sol_out_before_fee = (virtual_sol_reserves as u128).checked_sub(new_virtual_sol)?;
    let sol_out_before_fee = sol_out_before_fee.min(real_sol_reserves as u128) as u64;

    let eff_creator_bps = effective_creator_fee_bps(creator_fee_bps);
    let protocol_fee = calculate_fee(sol_out_before_fee, protocol_fee_bps)?;
    let creator_fee = calculate_fee(sol_out_before_fee, eff_creator_bps)?;
    let total_fee = protocol_fee.checked_add(creator_fee)?;
    let sol_out = sol_out_before_fee.checked_sub(total_fee)?;

    Some((sol_out, total_fee))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INITIAL_VIRTUAL_SOL: u64 = 30_000_000_000;
    const INITIAL_VIRTUAL_TOKENS: u64 = 1_073_000_000_000_000;
    const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000;
    const PROTOCOL_FEE_BPS: u16 = 45;

    #[test]
    fn test_calculate_fee() {
        assert_eq!(calculate_fee(10_000, 45), Some(45));
        assert_eq!(calculate_fee(1_000_000_000, 45), Some(4_500_000));
        assert_eq!(calculate_fee(0, 45), Some(0));
    }

    #[test]
    fn test_effective_creator_fee() {
        assert_eq!(effective_creator_fee_bps(10000), 30); // 100% of 30 bps
        assert_eq!(effective_creator_fee_bps(5000), 15); // 50% of 30 bps
        assert_eq!(effective_creator_fee_bps(0), 0); // 0% of 30 bps
    }

    #[test]
    fn test_buy_basic() {
        let (tokens, fee) = calculate_buy(
            1_000_000_000, // 1 SOL
            INITIAL_VIRTUAL_SOL,
            INITIAL_VIRTUAL_TOKENS,
            TOTAL_SUPPLY,
            PROTOCOL_FEE_BPS,
            10000, // 100% creator fee multiplier
        )
        .unwrap();
        assert!(tokens > 0);
        assert!(fee > 0);
        assert!(tokens < TOTAL_SUPPLY);
    }

    #[test]
    fn test_sell_basic() {
        let (sol_out, fee) = calculate_sell(
            1_000_000_000_000,                            // 1M tokens (with 6 decimals)
            INITIAL_VIRTUAL_SOL + 5_000_000_000,          // some SOL already in
            INITIAL_VIRTUAL_TOKENS - 100_000_000_000_000, // some tokens sold
            5_000_000_000,                                // 5 SOL real reserves
            PROTOCOL_FEE_BPS,
            10000,
        )
        .unwrap();
        assert!(sol_out > 0);
        assert!(fee > 0);
    }

    #[test]
    fn test_buy_sell_no_profit() {
        // Buy then immediately sell — should not produce profit
        let sol_in = 1_000_000_000u64; // 1 SOL
        let vsol = INITIAL_VIRTUAL_SOL;
        let vtok = INITIAL_VIRTUAL_TOKENS;
        let real_tok = TOTAL_SUPPLY;
        let real_sol = 0u64;

        let (tokens_out, buy_fee) =
            calculate_buy(sol_in, vsol, vtok, real_tok, PROTOCOL_FEE_BPS, 10000).unwrap();

        let sol_after_buy_fee = sol_in - buy_fee;
        let new_vsol = vsol + sol_after_buy_fee;
        let new_vtok = vtok - tokens_out;
        let new_real_sol = real_sol + sol_after_buy_fee;
        let _new_real_tok = real_tok - tokens_out;

        let (sol_back, _sell_fee) = calculate_sell(
            tokens_out,
            new_vsol,
            new_vtok,
            new_real_sol,
            PROTOCOL_FEE_BPS,
            10000,
        )
        .unwrap();

        // Should get back less than invested (fees on both sides)
        assert!(sol_back < sol_in);
    }
}
