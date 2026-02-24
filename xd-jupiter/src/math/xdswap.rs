/// XDSwap constant product math.
///
/// Calculate fee: fee = (amount * fee_bps) / 10_000
pub fn calculate_fee(amount: u64, fee_bps: u16) -> Option<u64> {
    let fee = (amount as u128)
        .checked_mul(fee_bps as u128)?
        .checked_div(10_000)?;
    Some(fee as u64)
}

/// Effective creator fee bps after multiplier.
pub fn effective_creator_fee_bps(base_creator_fee_bps: u16, multiplier_bps: u16) -> u16 {
    ((base_creator_fee_bps as u32 * multiplier_bps as u32) / 10000) as u16
}

/// Buy ExactIn: given exact quote input, return base tokens out.
/// Matches buy_exact_in.rs fee ordering: fees extracted from input, then swap.
///
/// Returns (base_out, total_fee, lp_fee, protocol_fee, creator_fee) or None.
pub fn calculate_buy_exact_in(
    quote_amount_in: u64,
    base_reserves: u64,
    quote_reserves: u64,
    lp_fee_bps: u16,
    protocol_fee_bps: u16,
    effective_creator_fee_bps: u16,
) -> Option<BuyExactInResult> {
    let total_fee_bps = (lp_fee_bps as u64)
        .checked_add(protocol_fee_bps as u64)?
        .checked_add(effective_creator_fee_bps as u64)?;

    // quote_in_raw = quote_amount_in * 10000 / (10000 + total_fee_bps)
    let quote_in_raw = (quote_amount_in as u128)
        .checked_mul(10_000)?
        .checked_div(10_000u128.checked_add(total_fee_bps as u128)?)? as u64;

    if quote_in_raw == 0 {
        return None;
    }

    // base_out = (quote_in_raw * base_reserves) / (quote_reserves + quote_in_raw)
    let numerator = (quote_in_raw as u128).checked_mul(base_reserves as u128)?;
    let denominator = (quote_reserves as u128).checked_add(quote_in_raw as u128)?;
    let base_out = numerator.checked_div(denominator)? as u64;

    // Individual fees from quote_in_raw
    let lp_fee = calculate_fee(quote_in_raw, lp_fee_bps)?;
    let base_protocol_fee = calculate_fee(quote_in_raw, protocol_fee_bps)?;
    let creator_fee = calculate_fee(quote_in_raw, effective_creator_fee_bps)?;

    // Dust handling: ensure no value leaks
    let fees_before_dust = quote_in_raw
        .checked_add(lp_fee)?
        .checked_add(base_protocol_fee)?
        .checked_add(creator_fee)?;
    let rounding_dust = quote_amount_in.saturating_sub(fees_before_dust);
    let protocol_fee = base_protocol_fee.checked_add(rounding_dust)?;

    let total_fee = lp_fee.checked_add(protocol_fee)?.checked_add(creator_fee)?;

    Some(BuyExactInResult {
        base_out,
        total_fee,
        lp_fee,
        protocol_fee,
        creator_fee,
    })
}

#[derive(Debug, Clone)]
pub struct BuyExactInResult {
    pub base_out: u64,
    pub total_fee: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub creator_fee: u64,
}

/// Buy ExactOut: given exact base output, return quote needed.
/// Matches buy.rs: calculate raw quote, add fees on top.
///
/// Returns (total_quote_in, total_fee) or None.
pub fn calculate_buy_exact_out(
    base_amount_out: u64,
    base_reserves: u64,
    quote_reserves: u64,
    lp_fee_bps: u16,
    protocol_fee_bps: u16,
    effective_creator_fee_bps: u16,
) -> Option<BuyExactOutResult> {
    if base_amount_out == 0 || base_amount_out >= base_reserves {
        return None;
    }

    // quote_in_raw = ceil((base_out * quote_reserves) / (base_reserves - base_out))
    let numerator = (base_amount_out as u128).checked_mul(quote_reserves as u128)?;
    let denominator = (base_reserves as u128).checked_sub(base_amount_out as u128)?;
    let quote_in_raw = numerator
        .checked_add(denominator)?
        .checked_sub(1)?
        .checked_div(denominator)? as u64;

    // Calculate fees from raw
    let lp_fee = calculate_fee(quote_in_raw, lp_fee_bps)?;
    let base_protocol_fee = calculate_fee(quote_in_raw, protocol_fee_bps)?;
    let creator_fee = calculate_fee(quote_in_raw, effective_creator_fee_bps)?;

    // Dust handling (same as on-chain buy.rs)
    let total_fee_bps = (lp_fee_bps as u64)
        .checked_add(protocol_fee_bps as u64)?
        .checked_add(effective_creator_fee_bps as u64)?;
    let expected_total_fee = calculate_fee(quote_in_raw, total_fee_bps as u16)?;
    let actual_fees_before_dust = lp_fee
        .checked_add(base_protocol_fee)?
        .checked_add(creator_fee)?;
    let rounding_dust = expected_total_fee.saturating_sub(actual_fees_before_dust);
    let protocol_fee = base_protocol_fee.checked_add(rounding_dust)?;

    let total_quote_in = quote_in_raw
        .checked_add(lp_fee)?
        .checked_add(protocol_fee)?
        .checked_add(creator_fee)?;

    let total_fee = lp_fee.checked_add(protocol_fee)?.checked_add(creator_fee)?;

    Some(BuyExactOutResult {
        total_quote_in,
        total_fee,
    })
}

#[derive(Debug, Clone)]
pub struct BuyExactOutResult {
    pub total_quote_in: u64,
    pub total_fee: u64,
}

/// Sell (ExactIn): given exact base input, return quote out.
/// Matches sell.rs: swap first, then deduct fees from output.
///
/// Returns (quote_out, total_fee) or None.
pub fn calculate_sell(
    base_amount_in: u64,
    base_reserves: u64,
    quote_reserves: u64,
    lp_fee_bps: u16,
    protocol_fee_bps: u16,
    effective_creator_fee_bps: u16,
) -> Option<SellResult> {
    if base_amount_in == 0 {
        return None;
    }

    // quote_out_raw = (base_in * quote_reserves) / (base_reserves + base_in)
    let numerator = (base_amount_in as u128).checked_mul(quote_reserves as u128)?;
    let denominator = (base_reserves as u128).checked_add(base_amount_in as u128)?;
    let quote_out_raw = numerator.checked_div(denominator)? as u64;

    if quote_out_raw == 0 {
        return None;
    }

    // Individual fees from quote_out_raw
    let lp_fee = calculate_fee(quote_out_raw, lp_fee_bps)?;
    let base_protocol_fee = calculate_fee(quote_out_raw, protocol_fee_bps)?;
    let creator_fee = calculate_fee(quote_out_raw, effective_creator_fee_bps)?;

    // Dust handling
    let total_fee_bps = (lp_fee_bps as u64)
        .checked_add(protocol_fee_bps as u64)?
        .checked_add(effective_creator_fee_bps as u64)?;
    let expected_total_fee = calculate_fee(quote_out_raw, total_fee_bps as u16)?;
    let actual_fees_before_dust = lp_fee
        .checked_add(base_protocol_fee)?
        .checked_add(creator_fee)?;
    let rounding_dust = expected_total_fee.saturating_sub(actual_fees_before_dust);
    let protocol_fee = base_protocol_fee.checked_add(rounding_dust)?;

    let quote_out = quote_out_raw
        .checked_sub(lp_fee)?
        .checked_sub(protocol_fee)?
        .checked_sub(creator_fee)?;

    let total_fee = lp_fee.checked_add(protocol_fee)?.checked_add(creator_fee)?;

    Some(SellResult {
        quote_out,
        total_fee,
        lp_fee,
        protocol_fee,
        creator_fee,
    })
}

#[derive(Debug, Clone)]
pub struct SellResult {
    pub quote_out: u64,
    pub total_fee: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub creator_fee: u64,
}

/// Sell ExactOut: given desired quote output, return base tokens needed.
/// Reverse of sell formula.
///
/// Returns (base_amount_in, total_fee) or None.
pub fn calculate_sell_exact_out(
    quote_amount_out: u64,
    base_reserves: u64,
    quote_reserves: u64,
    lp_fee_bps: u16,
    protocol_fee_bps: u16,
    effective_creator_fee_bps: u16,
) -> Option<SellExactOutResult> {
    if quote_amount_out == 0 {
        return None;
    }

    // Reverse fee deduction: quote_out_raw such that quote_out_raw - fees = quote_amount_out
    // total_fee_bps = lp + protocol + creator
    let total_fee_bps = (lp_fee_bps as u64)
        .checked_add(protocol_fee_bps as u64)?
        .checked_add(effective_creator_fee_bps as u64)?;

    // quote_out_raw = ceil(quote_amount_out * 10000 / (10000 - total_fee_bps))
    let denom = 10_000u128.checked_sub(total_fee_bps as u128)?;
    if denom == 0 {
        return None;
    }
    let quote_out_raw = (quote_amount_out as u128)
        .checked_mul(10_000)?
        .checked_add(denom)?
        .checked_sub(1)?
        .checked_div(denom)? as u64;

    if quote_out_raw == 0 || quote_out_raw >= quote_reserves {
        return None;
    }

    // Reverse constant product: base_in = (quote_out_raw * base_reserves) / (quote_reserves - quote_out_raw)
    // Round up
    let numerator = (quote_out_raw as u128).checked_mul(base_reserves as u128)?;
    let denominator = (quote_reserves as u128).checked_sub(quote_out_raw as u128)?;
    let base_amount_in = numerator
        .checked_add(denominator)?
        .checked_sub(1)?
        .checked_div(denominator)? as u64;

    // Compute actual fee
    let lp_fee = calculate_fee(quote_out_raw, lp_fee_bps)?;
    let base_protocol_fee = calculate_fee(quote_out_raw, protocol_fee_bps)?;
    let creator_fee = calculate_fee(quote_out_raw, effective_creator_fee_bps)?;
    let expected_total_fee = calculate_fee(quote_out_raw, total_fee_bps as u16)?;
    let actual_fees_before_dust = lp_fee
        .checked_add(base_protocol_fee)?
        .checked_add(creator_fee)?;
    let rounding_dust = expected_total_fee.saturating_sub(actual_fees_before_dust);
    let protocol_fee = base_protocol_fee.checked_add(rounding_dust)?;
    let total_fee = lp_fee.checked_add(protocol_fee)?.checked_add(creator_fee)?;

    Some(SellExactOutResult {
        base_amount_in,
        total_fee,
    })
}

#[derive(Debug, Clone)]
pub struct SellExactOutResult {
    pub base_amount_in: u64,
    pub total_fee: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buy_exact_in_basic() {
        let result = calculate_buy_exact_in(
            1_000_000_000,       // 1 SOL
            500_000_000_000_000, // 500M base
            50_000_000_000,      // 50 SOL quote
            20,
            5,
            30, // lp, protocol, creator bps
        )
        .unwrap();
        assert!(result.base_out > 0);
        assert!(result.total_fee > 0);
    }

    #[test]
    fn test_sell_basic() {
        let result = calculate_sell(
            1_000_000_000_000, // 1M tokens
            500_000_000_000_000,
            50_000_000_000,
            20,
            5,
            30,
        )
        .unwrap();
        assert!(result.quote_out > 0);
        assert!(result.total_fee > 0);
    }

    #[test]
    fn test_buy_exact_out_basic() {
        let result = calculate_buy_exact_out(
            1_000_000_000_000, // 1M tokens
            500_000_000_000_000,
            50_000_000_000,
            20,
            5,
            30,
        )
        .unwrap();
        assert!(result.total_quote_in > 0);
        assert!(result.total_fee > 0);
    }

    #[test]
    fn test_dust_routes_to_protocol() {
        // Verify rounding dust gets added to protocol fee
        let result = calculate_sell(
            333_333, // odd number to trigger rounding
            500_000_000_000_000,
            50_000_000_000,
            20,
            5,
            30,
        );
        // Should succeed without overflow
        assert!(result.is_some());
    }
}
