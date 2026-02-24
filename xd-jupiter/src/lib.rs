pub mod launchpad_amm;
pub mod math;
pub mod state;
pub mod xdswap_amm;

use jupiter_amm_interface::{single_program_amm, AmmProgramIdToLabel, SingleProgramAmm};
pub use launchpad_amm::LaunchpadAmm;
pub use xdswap_amm::XDSwapAmm;

use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

/// Launchpad bonding curve program ID
pub const LAUNCHPAD_PROGRAM_ID: Pubkey = pubkey!("XDBC3FsUpjDnYCcPBEgniLo4M13Wsiu8yLbcwcz2zqV");

/// XDSwap AMM program ID
pub const XDSWAP_PROGRAM_ID: Pubkey = pubkey!("XDSwtQ2qNdjT4HsToizoAUwz3wAWL5nAhChQTEcv1Uh");

/// XDSwap devnet program ID used by current devnet pools.
pub const XDSWAP_DEVNET_PROGRAM_ID: Pubkey = pubkey!("KYtn3hdA1sf2UfUNsbqjz2hAFHPgL95uzbpXZMR3zgi");

/// Native SOL mint (used as reserve mint for bonding curve)
pub const NATIVE_SOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

/// Token-2022 program ID
pub const TOKEN_2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Standard Token program ID
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Associated Token program ID
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// System program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");

/// Derive associated token address for a given wallet, mint, and token program.
pub fn get_associated_token_address(
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

single_program_amm!(LaunchpadAmm, LAUNCHPAD_PROGRAM_ID, "xd-launchpad");

impl AmmProgramIdToLabel for XDSwapAmm {
    const PROGRAM_ID_TO_LABELS: &[(Pubkey, &'static str)] = &[
        (XDSWAP_PROGRAM_ID, "xdswap"),
        (XDSWAP_DEVNET_PROGRAM_ID, "xdswap"),
    ];
}
