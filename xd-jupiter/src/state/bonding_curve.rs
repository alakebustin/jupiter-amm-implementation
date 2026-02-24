use anyhow::{ensure, Result};
use solana_sdk::pubkey::Pubkey;

use sha2::{Digest, Sha256};

/// Expected account size for V6 BondingCurve (current mainnet layout).
pub const BONDING_CURVE_SIZE: usize = 192;

/// Anchor discriminator for BondingCurve: sha256("account:BondingCurve")[0..8]
fn bonding_curve_discriminator() -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"account:BondingCurve");
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Decoded BondingCurve state (V6, 192 bytes).
#[derive(Debug, Clone)]
pub struct BondingCurve {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub is_migrated: bool,
    pub creator_fee_accumulated: u64,
    // FeeDistributionConfig (42 bytes) — we skip parsing fields we don't need for quoting
    pub created_at: i64,
    pub bump: u8,
    pub creator_wallet_changed: bool,
    pub creator_fee_bps: u16,
    pub creator_fees_claimed: u64,
    pub total_fees_ever: u64,
}

impl BondingCurve {
    /// Decode from raw account data bytes (including 8-byte Anchor discriminator).
    pub fn decode(data: &[u8]) -> Result<Self> {
        ensure!(
            data.len() >= BONDING_CURVE_SIZE,
            "BondingCurve data too short: {} < {}",
            data.len(),
            BONDING_CURVE_SIZE
        );

        let expected_disc = bonding_curve_discriminator();
        ensure!(
            data[..8] == expected_disc,
            "BondingCurve discriminator mismatch"
        );

        let d = &data[8..]; // skip discriminator

        let mint = Pubkey::try_from(&d[0..32]).unwrap();
        let creator = Pubkey::try_from(&d[32..64]).unwrap();
        let virtual_sol_reserves = u64::from_le_bytes(d[64..72].try_into().unwrap());
        let virtual_token_reserves = u64::from_le_bytes(d[72..80].try_into().unwrap());
        let real_sol_reserves = u64::from_le_bytes(d[80..88].try_into().unwrap());
        let real_token_reserves = u64::from_le_bytes(d[88..96].try_into().unwrap());
        let token_total_supply = u64::from_le_bytes(d[96..104].try_into().unwrap());
        let complete = d[104] != 0;
        let is_migrated = d[105] != 0;
        let creator_fee_accumulated = u64::from_le_bytes(d[106..114].try_into().unwrap());
        // Skip FeeDistributionConfig (42 bytes) at d[114..156]
        let created_at = i64::from_le_bytes(d[156..164].try_into().unwrap());
        let bump = d[164];
        let creator_wallet_changed = d[165] != 0;
        let creator_fee_bps = u16::from_le_bytes(d[166..168].try_into().unwrap());
        let creator_fees_claimed = u64::from_le_bytes(d[168..176].try_into().unwrap());
        let total_fees_ever = u64::from_le_bytes(d[176..184].try_into().unwrap());

        Ok(Self {
            mint,
            creator,
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves,
            token_total_supply,
            complete,
            is_migrated,
            creator_fee_accumulated,
            created_at,
            bump,
            creator_wallet_changed,
            creator_fee_bps,
            creator_fees_claimed,
            total_fees_ever,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminator_is_stable() {
        let disc = bonding_curve_discriminator();
        // Anchor discriminator for "account:BondingCurve"
        // This should remain constant — if it changes, our parsing breaks.
        assert_eq!(disc.len(), 8);
        // Smoke test: discriminator is not all zeros
        assert_ne!(disc, [0u8; 8]);
    }

    #[test]
    fn test_decode_rejects_short_data() {
        let data = vec![0u8; 100];
        assert!(BondingCurve::decode(&data).is_err());
    }

    #[test]
    fn test_decode_rejects_bad_discriminator() {
        let mut data = vec![0u8; BONDING_CURVE_SIZE];
        data[0..8].copy_from_slice(&[0xFF; 8]);
        assert!(BondingCurve::decode(&data).is_err());
    }
}
