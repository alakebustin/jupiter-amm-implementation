use anyhow::{ensure, Result};
use solana_sdk::pubkey::Pubkey;

use sha2::{Digest, Sha256};

/// Expected account size for current XDSwap Pool (313 bytes).
pub const POOL_SIZE: usize = 313;

/// Anchor discriminator for Pool: sha256("account:Pool")[0..8]
fn pool_discriminator() -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"account:Pool");
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Decoded XDSwap Pool state (313 bytes, current mainnet layout).
#[derive(Debug, Clone)]
pub struct XDSwapPool {
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_reserves: u64,
    pub quote_reserves: u64,
    pub lp_supply: u64,
    pub coin_creator: Pubkey,
    pub creator_fee_accumulated: u64,
    pub total_creator_fees_ever: u64,
    pub creator_fees_claimed: u64,
    pub protocol_fee_accumulated: u64, // deprecated, kept for layout
    pub is_active: bool,
    pub created_at: i64,
    pub bump: u8,
    pub lp_mint_bump: u8,
    pub base_vault_bump: u8,
    pub quote_vault_bump: u8,
    pub creator_fee_multiplier_bps: u16,
    pub pending_social_fees: u64,
}

impl XDSwapPool {
    /// Decode from raw account data bytes (including 8-byte Anchor discriminator).
    pub fn decode(data: &[u8]) -> Result<Self> {
        ensure!(
            data.len() >= POOL_SIZE,
            "XDSwapPool data too short: {} < {}",
            data.len(),
            POOL_SIZE
        );

        let expected_disc = pool_discriminator();
        ensure!(
            data[..8] == expected_disc,
            "XDSwapPool discriminator mismatch"
        );

        let d = &data[8..]; // skip discriminator
        let mut offset = 0;

        let index = u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let creator = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let base_mint = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let quote_mint = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let lp_mint = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let base_vault = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let quote_vault = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let base_reserves = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let quote_reserves = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let lp_supply = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let coin_creator = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let creator_fee_accumulated = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let total_creator_fees_ever = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let creator_fees_claimed = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let protocol_fee_accumulated =
            u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let is_active = d[offset] != 0;
        offset += 1;

        let created_at = i64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let bump = d[offset];
        offset += 1;

        let lp_mint_bump = d[offset];
        offset += 1;

        let base_vault_bump = d[offset];
        offset += 1;

        let quote_vault_bump = d[offset];
        offset += 1;

        let creator_fee_multiplier_bps =
            u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let pending_social_fees = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            index,
            creator,
            base_mint,
            quote_mint,
            lp_mint,
            base_vault,
            quote_vault,
            base_reserves,
            quote_reserves,
            lp_supply,
            coin_creator,
            creator_fee_accumulated,
            total_creator_fees_ever,
            creator_fees_claimed,
            protocol_fee_accumulated,
            is_active,
            created_at,
            bump,
            lp_mint_bump,
            base_vault_bump,
            quote_vault_bump,
            creator_fee_multiplier_bps,
            pending_social_fees,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_discriminator_is_stable() {
        let disc = pool_discriminator();
        assert_eq!(disc.len(), 8);
        assert_ne!(disc, [0u8; 8]);
    }

    #[test]
    fn test_decode_rejects_short_data() {
        let data = vec![0u8; 100];
        assert!(XDSwapPool::decode(&data).is_err());
    }
}
