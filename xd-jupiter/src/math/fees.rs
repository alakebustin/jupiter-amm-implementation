/// Dynamic fee tiers for XDSwap, based on market cap.
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[derive(Clone, Copy)]
pub struct FeeTier {
    pub market_cap_threshold: u64,
    pub creator_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub lp_fee_bps: u16,
}

pub const FEE_TIERS: [FeeTier; 26] = [
    FeeTier {
        market_cap_threshold: 0,
        creator_fee_bps: 30,
        protocol_fee_bps: 45,
        lp_fee_bps: 2,
    },
    FeeTier {
        market_cap_threshold: 420 * LAMPORTS_PER_SOL,
        creator_fee_bps: 95,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 1470 * LAMPORTS_PER_SOL,
        creator_fee_bps: 90,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 2460 * LAMPORTS_PER_SOL,
        creator_fee_bps: 85,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 3440 * LAMPORTS_PER_SOL,
        creator_fee_bps: 80,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 4420 * LAMPORTS_PER_SOL,
        creator_fee_bps: 75,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 9820 * LAMPORTS_PER_SOL,
        creator_fee_bps: 70,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 14740 * LAMPORTS_PER_SOL,
        creator_fee_bps: 65,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 19650 * LAMPORTS_PER_SOL,
        creator_fee_bps: 60,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 24560 * LAMPORTS_PER_SOL,
        creator_fee_bps: 55,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 29470 * LAMPORTS_PER_SOL,
        creator_fee_bps: 50,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 34380 * LAMPORTS_PER_SOL,
        creator_fee_bps: 45,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 39300 * LAMPORTS_PER_SOL,
        creator_fee_bps: 40,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 44210 * LAMPORTS_PER_SOL,
        creator_fee_bps: 35,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 49120 * LAMPORTS_PER_SOL,
        creator_fee_bps: 30,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 54030 * LAMPORTS_PER_SOL,
        creator_fee_bps: 28,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 58940 * LAMPORTS_PER_SOL,
        creator_fee_bps: 25,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 63860 * LAMPORTS_PER_SOL,
        creator_fee_bps: 23,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 68770 * LAMPORTS_PER_SOL,
        creator_fee_bps: 20,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 73681 * LAMPORTS_PER_SOL,
        creator_fee_bps: 18,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 78590 * LAMPORTS_PER_SOL,
        creator_fee_bps: 15,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 83500 * LAMPORTS_PER_SOL,
        creator_fee_bps: 13,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 88400 * LAMPORTS_PER_SOL,
        creator_fee_bps: 10,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 93330 * LAMPORTS_PER_SOL,
        creator_fee_bps: 8,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    FeeTier {
        market_cap_threshold: 98240 * LAMPORTS_PER_SOL,
        creator_fee_bps: 5,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
    // Sentinel
    FeeTier {
        market_cap_threshold: u64::MAX,
        creator_fee_bps: 5,
        protocol_fee_bps: 5,
        lp_fee_bps: 20,
    },
];

/// Total token supply: 1 billion with 6 decimals
const TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;

/// Calculate market cap from pool reserves.
/// market_cap = (quote_reserves * TOTAL_SUPPLY) / base_reserves
pub fn calculate_market_cap(base_reserves: u64, quote_reserves: u64) -> u64 {
    if base_reserves == 0 {
        return 0;
    }
    let market_cap = (quote_reserves as u128)
        .saturating_mul(TOTAL_SUPPLY)
        .checked_div(base_reserves as u128)
        .unwrap_or(0);
    if market_cap > u64::MAX as u128 {
        u64::MAX
    } else {
        market_cap as u64
    }
}

/// Get fee tier for a given market cap.
/// Returns (creator_fee_bps, protocol_fee_bps, lp_fee_bps).
pub fn get_fees_for_market_cap(market_cap: u64) -> (u16, u16, u16) {
    let mut selected_tier = &FEE_TIERS[0];
    for tier in FEE_TIERS.iter() {
        if market_cap >= tier.market_cap_threshold {
            selected_tier = tier;
        } else {
            break;
        }
    }
    (
        selected_tier.creator_fee_bps,
        selected_tier.protocol_fee_bps,
        selected_tier.lp_fee_bps,
    )
}

/// Get dynamic fees for a pool based on its current reserves.
pub fn get_dynamic_fees(base_reserves: u64, quote_reserves: u64) -> (u16, u16, u16) {
    let market_cap = calculate_market_cap(base_reserves, quote_reserves);
    get_fees_for_market_cap(market_cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowest_tier() {
        let (creator, protocol, lp) = get_fees_for_market_cap(0);
        assert_eq!(creator, 30);
        assert_eq!(protocol, 45);
        assert_eq!(lp, 2);
    }

    #[test]
    fn test_after_420_sol() {
        let (creator, protocol, lp) = get_fees_for_market_cap(500 * LAMPORTS_PER_SOL);
        assert_eq!(creator, 95);
        assert_eq!(protocol, 5);
        assert_eq!(lp, 20);
    }

    #[test]
    fn test_highest_tier() {
        let (creator, protocol, lp) = get_fees_for_market_cap(100_000 * LAMPORTS_PER_SOL);
        assert_eq!(creator, 5);
        assert_eq!(protocol, 5);
        assert_eq!(lp, 20);
    }

    #[test]
    fn test_market_cap_calculation() {
        let base_reserves: u64 = 1_000_000_000_000; // 1M tokens
        let quote_reserves: u64 = 10_000_000_000; // 10 SOL
        let market_cap = calculate_market_cap(base_reserves, quote_reserves);
        assert_eq!(market_cap, 10_000 * LAMPORTS_PER_SOL);
    }
}
