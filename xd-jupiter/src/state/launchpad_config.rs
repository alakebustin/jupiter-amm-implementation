use anyhow::{ensure, Result};
use solana_sdk::pubkey::Pubkey;

use sha2::{Digest, Sha256};

/// Expected account size for Launchpad GlobalConfig.
pub const LAUNCHPAD_CONFIG_SIZE: usize = 398;

/// Anchor discriminator for GlobalConfig: sha256("account:GlobalConfig")[0..8]
/// Note: Both launchpad and xdswap have "GlobalConfig" but different program owners.
fn global_config_discriminator() -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"account:GlobalConfig");
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Decoded Launchpad GlobalConfig state (398 bytes).
#[derive(Debug, Clone)]
pub struct LaunchpadGlobalConfig {
    pub authority: Pubkey,
    pub fee_recipients: [Pubkey; 8],
    pub migration_authority: Pubkey,
    pub distribution_authority: Pubkey,
    pub creation_fee: u64,
    pub protocol_fee_bps: u16,
    pub creator_fee_bps: u16,
    pub migration_fee: u64,
    pub migration_threshold: u64,
    pub paused: bool,
    pub total_tokens_created: u64,
    pub bump: u8,
}

impl LaunchpadGlobalConfig {
    /// Decode from raw account data bytes (including 8-byte Anchor discriminator).
    pub fn decode(data: &[u8]) -> Result<Self> {
        ensure!(
            data.len() >= LAUNCHPAD_CONFIG_SIZE,
            "LaunchpadGlobalConfig data too short: {} < {}",
            data.len(),
            LAUNCHPAD_CONFIG_SIZE
        );

        let expected_disc = global_config_discriminator();
        ensure!(
            data[..8] == expected_disc,
            "LaunchpadGlobalConfig discriminator mismatch"
        );

        let d = &data[8..]; // skip discriminator
        let mut offset = 0;

        let authority = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let mut fee_recipients = [Pubkey::default(); 8];
        for recipient in &mut fee_recipients {
            *recipient = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
            offset += 32;
        }

        let migration_authority = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let distribution_authority = Pubkey::try_from(&d[offset..offset + 32]).unwrap();
        offset += 32;

        let creation_fee = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let protocol_fee_bps = u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let creator_fee_bps = u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let migration_fee = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let migration_threshold = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let paused = d[offset] != 0;
        offset += 1;

        let total_tokens_created = u64::from_le_bytes(d[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let bump = d[offset];

        Ok(Self {
            authority,
            fee_recipients,
            migration_authority,
            distribution_authority,
            creation_fee,
            protocol_fee_bps,
            creator_fee_bps,
            migration_fee,
            migration_threshold,
            paused,
            total_tokens_created,
            bump,
        })
    }
}
