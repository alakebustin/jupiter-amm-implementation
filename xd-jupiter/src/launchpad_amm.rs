use anyhow::{bail, Result};
use jupiter_amm_interface::{
    try_get_account_data, AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap,
    SwapAndAccountMetas, SwapMode, SwapParams,
};
use rust_decimal::Decimal;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

use crate::math::bonding_curve as bc_math;
use crate::state::{BondingCurve, LaunchpadGlobalConfig};
use crate::{
    get_associated_token_address, ASSOCIATED_TOKEN_PROGRAM_ID, LAUNCHPAD_PROGRAM_ID,
    NATIVE_SOL_MINT, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
};

/// Launchpad buy instruction discriminator
#[allow(dead_code)]
const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];

/// Launchpad sell instruction discriminator
#[allow(dead_code)]
const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

/// Jupiter AMM implementation for XD Launchpad bonding curves.
///
/// Handles pre-migration tokens trading on the bonding curve with native SOL.
/// Only supports ExactIn swaps (on-chain only has ExactIn buy/sell instructions).
pub struct LaunchpadAmm {
    key: Pubkey,
    mint: Pubkey,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_sol_reserves: u64,
    real_token_reserves: u64,
    complete: bool,
    is_migrated: bool,
    creator_fee_bps: u16,
    bump: u8,
    global_config_key: Pubkey,
    protocol_fee_bps: u16,
    fee_recipient: Pubkey,
    paused: bool,
    curve_token_account: Pubkey,
}

impl Clone for LaunchpadAmm {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            mint: self.mint,
            virtual_sol_reserves: self.virtual_sol_reserves,
            virtual_token_reserves: self.virtual_token_reserves,
            real_sol_reserves: self.real_sol_reserves,
            real_token_reserves: self.real_token_reserves,
            complete: self.complete,
            is_migrated: self.is_migrated,
            creator_fee_bps: self.creator_fee_bps,
            bump: self.bump,
            global_config_key: self.global_config_key,
            protocol_fee_bps: self.protocol_fee_bps,
            fee_recipient: self.fee_recipient,
            paused: self.paused,
            curve_token_account: self.curve_token_account,
        }
    }
}

impl LaunchpadAmm {
    fn derive_global_config() -> Pubkey {
        Pubkey::find_program_address(&[b"global_config"], &LAUNCHPAD_PROGRAM_ID).0
    }

    fn is_buy_pair(input_mint: Pubkey, output_mint: Pubkey, token_mint: Pubkey) -> bool {
        input_mint == NATIVE_SOL_MINT && output_mint == token_mint
    }

    fn is_sell_pair(input_mint: Pubkey, output_mint: Pubkey, token_mint: Pubkey) -> bool {
        input_mint == token_mint && output_mint == NATIVE_SOL_MINT
    }
}

impl Amm for LaunchpadAmm {
    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> Result<Self> {
        let curve = BondingCurve::decode(&keyed_account.account.data)?;
        let global_config_key = Self::derive_global_config();
        let curve_token_account =
            get_associated_token_address(&keyed_account.key, &curve.mint, &TOKEN_2022_PROGRAM_ID);

        Ok(Self {
            key: keyed_account.key,
            mint: curve.mint,
            virtual_sol_reserves: curve.virtual_sol_reserves,
            virtual_token_reserves: curve.virtual_token_reserves,
            real_sol_reserves: curve.real_sol_reserves,
            real_token_reserves: curve.real_token_reserves,
            complete: curve.complete,
            is_migrated: curve.is_migrated,
            creator_fee_bps: curve.creator_fee_bps,
            bump: curve.bump,
            global_config_key,
            protocol_fee_bps: 0,
            fee_recipient: Pubkey::default(),
            paused: true,
            curve_token_account,
        })
    }

    fn label(&self) -> String {
        "XD Launchpad".into()
    }

    fn program_id(&self) -> Pubkey {
        LAUNCHPAD_PROGRAM_ID
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        vec![NATIVE_SOL_MINT, self.mint]
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![self.key, self.global_config_key]
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let curve_data = try_get_account_data(account_map, &self.key)?;
        let curve = BondingCurve::decode(curve_data)?;

        self.virtual_sol_reserves = curve.virtual_sol_reserves;
        self.virtual_token_reserves = curve.virtual_token_reserves;
        self.real_sol_reserves = curve.real_sol_reserves;
        self.real_token_reserves = curve.real_token_reserves;
        self.complete = curve.complete;
        self.is_migrated = curve.is_migrated;
        self.creator_fee_bps = curve.creator_fee_bps;

        let config_data = try_get_account_data(account_map, &self.global_config_key)?;
        let config = LaunchpadGlobalConfig::decode(config_data)?;

        self.protocol_fee_bps = config.protocol_fee_bps;
        self.fee_recipient = config.fee_recipients[0];
        self.paused = config.paused;

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        if quote_params.swap_mode == SwapMode::ExactOut {
            bail!("LaunchpadAmm does not support ExactOut");
        }

        let is_buy =
            Self::is_buy_pair(quote_params.input_mint, quote_params.output_mint, self.mint);
        let is_sell =
            Self::is_sell_pair(quote_params.input_mint, quote_params.output_mint, self.mint);

        if !is_buy && !is_sell {
            bail!(
                "Invalid mint pair for LaunchpadAmm: input={} output={}",
                quote_params.input_mint,
                quote_params.output_mint
            );
        }

        if is_buy {
            let sol_amount = quote_params.amount;
            let (tokens_out, total_fee) = bc_math::calculate_buy(
                sol_amount,
                self.virtual_sol_reserves,
                self.virtual_token_reserves,
                self.real_token_reserves,
                self.protocol_fee_bps,
                self.creator_fee_bps,
            )
            .ok_or_else(|| anyhow::anyhow!("Buy calculation overflow"))?;

            let fee_pct = if sol_amount > 0 {
                Decimal::from(total_fee) / Decimal::from(sol_amount)
            } else {
                Decimal::ZERO
            };

            Ok(Quote {
                in_amount: sol_amount,
                out_amount: tokens_out,
                fee_amount: total_fee,
                fee_mint: NATIVE_SOL_MINT,
                fee_pct,
            })
        } else {
            let token_amount = quote_params.amount;
            let (sol_out, total_fee) = bc_math::calculate_sell(
                token_amount,
                self.virtual_sol_reserves,
                self.virtual_token_reserves,
                self.real_sol_reserves,
                self.protocol_fee_bps,
                self.creator_fee_bps,
            )
            .ok_or_else(|| anyhow::anyhow!("Sell calculation overflow"))?;

            let sol_before_fee = sol_out + total_fee;
            let fee_pct = if sol_before_fee > 0 {
                Decimal::from(total_fee) / Decimal::from(sol_before_fee)
            } else {
                Decimal::ZERO
            };

            Ok(Quote {
                in_amount: token_amount,
                out_amount: sol_out,
                fee_amount: total_fee,
                fee_mint: NATIVE_SOL_MINT,
                fee_pct,
            })
        }
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        if swap_params.swap_mode == SwapMode::ExactOut {
            bail!("LaunchpadAmm does not support ExactOut");
        }

        let is_buy = Self::is_buy_pair(
            swap_params.source_mint,
            swap_params.destination_mint,
            self.mint,
        );
        let is_sell = Self::is_sell_pair(
            swap_params.source_mint,
            swap_params.destination_mint,
            self.mint,
        );

        if !is_buy && !is_sell {
            bail!(
                "Invalid mint pair for LaunchpadAmm: source={} destination={}",
                swap_params.source_mint,
                swap_params.destination_mint
            );
        }

        let account_metas = if is_buy {
            vec![
                AccountMeta::new_readonly(self.global_config_key, false),
                AccountMeta::new(self.key, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new(self.curve_token_account, false),
                AccountMeta::new(swap_params.destination_token_account, false),
                AccountMeta::new(self.fee_recipient, false),
                AccountMeta::new(swap_params.token_transfer_authority, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
                AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            ]
        } else {
            vec![
                AccountMeta::new_readonly(self.global_config_key, false),
                AccountMeta::new(self.key, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new(self.curve_token_account, false),
                AccountMeta::new(swap_params.source_token_account, false),
                AccountMeta::new(self.fee_recipient, false),
                AccountMeta::new(swap_params.token_transfer_authority, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
                AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            ]
        };

        // Prepend program ID (Jupiter convention)
        let mut metas = vec![AccountMeta::new_readonly(LAUNCHPAD_PROGRAM_ID, false)];
        metas.extend(account_metas);

        // Wrapped SOL path for launchpad native SOL flows.
        let swap = if is_buy {
            Swap::PumpWrappedBuy
        } else {
            Swap::PumpWrappedSell
        };

        Ok(SwapAndAccountMetas {
            swap,
            account_metas: metas,
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn supports_exact_out(&self) -> bool {
        false
    }

    fn is_active(&self) -> bool {
        !self.complete && !self.is_migrated && !self.paused
    }

    fn get_accounts_len(&self) -> usize {
        11
    }
}
