use anyhow::{ensure, Result};
use solana_sdk::pubkey::Pubkey;

use sha2::{Digest, Sha256};

/// Expected account size for XDSwap GlobalConfig (312 bytes).
pub const XDSWAP_CONFIG_SIZE: usize = 312;

/// Anchor discriminator for GlobalConfig: sha256("account:GlobalConfig")[0..8]
/// Same discriminator as launchpad GlobalConfig — disambiguated by account owner (program ID).
fn global_config_discriminator() -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"account:GlobalConfig");
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Decoded XDSwap GlobalConfig state (312 bytes).
#[derive(Debug, Clone)]
pub struct XDSwapGlobalConfig {
    pub admin: Pubkey,
    pub lp_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub creator_fee_bps: u16,
    pub protocol_fee_recipients: [Pubkey; 8],
    pub total_pools_created: u64,
    pub paused: bool,
    pub bump: u8,
}

impl XDSwapGlobalConfig {
    /// Decode from raw account data bytes (including 8-byte Anchor discriminator).
    pub fn decode(data: &[u8]) -> Result<Self> {
        ensure!(
            data.len() >= XDSWAP_CONFIG_SIZE,
            "XDSwapGlobalConfig data too short: {} < {}",
            data.len(),
            XDSWAP_CONFIG_SIZE
        );

        let expected_disc = global_config_discriminator();
        ensure!(
            data[..8] == expected_disc,
            "XDSwapGlobalConfig discriminator mismatch"
        );

        let d = &data[8..]; // skip discriminator
        let mut offset = 0;

        let admin = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let lp_fee_bps = u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let protocol_fee_bps = u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let creator_fee_bps = u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let mut protocol_fee_recipients = [Pubkey::default(); 8];
        for recipient in &mut protocol_fee_recipients {
            *recipient = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
            offset += 32;
        }

        let total_pools_created = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let paused = d[offset] != 0;
        offset += 1;

        let bump = d[offset];

        Ok(Self {
            admin,
            lp_fee_bps,
            protocol_fee_bps,
            creator_fee_bps,
            protocol_fee_recipients,
            total_pools_created,
            paused,
            bump,
        })
    }
}
