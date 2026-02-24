use anyhow::{bail, Result};
use jupiter_amm_interface::{
    try_get_account_data, AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap,
    SwapAndAccountMetas, SwapMode, SwapParams,
};
use rust_decimal::Decimal;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

use crate::math::{fees as fee_math, xdswap as xd_math};
use crate::state::{XDSwapGlobalConfig, XDSwapPool};
use crate::{
    get_associated_token_address, ASSOCIATED_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID,
};

/// Jupiter AMM implementation for XDSwap pools.
///
/// Handles post-migration tokens trading on the XDSwap AMM with dynamic fees.
/// Supports both ExactIn and ExactOut swaps.
pub struct XDSwapAmm {
    program_id: Pubkey,
    key: Pubkey,
    index: u16,
    creator: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    base_reserves: u64,
    quote_reserves: u64,
    is_active: bool,
    bump: u8,
    creator_fee_multiplier_bps: u16,
    global_config_key: Pubkey,
    protocol_fee_recipient: Pubkey,
    protocol_fee_recipient_ata: Pubkey,
    quote_token_program: Pubkey,
    paused: bool,
    /// Whether this pool can be routed through.
    /// Set to false if quote_mint owner is unrecognized or fee ATA is unresolvable.
    /// When false, both quote() and get_swap_and_account_metas() return Err.
    routable: bool,
}

impl Clone for XDSwapAmm {
    fn clone(&self) -> Self {
        Self {
            program_id: self.program_id,
            key: self.key,
            index: self.index,
            creator: self.creator,
            base_mint: self.base_mint,
            quote_mint: self.quote_mint,
            base_vault: self.base_vault,
            quote_vault: self.quote_vault,
            base_reserves: self.base_reserves,
            quote_reserves: self.quote_reserves,
            is_active: self.is_active,
            bump: self.bump,
            creator_fee_multiplier_bps: self.creator_fee_multiplier_bps,
            global_config_key: self.global_config_key,
            protocol_fee_recipient: self.protocol_fee_recipient,
            protocol_fee_recipient_ata: self.protocol_fee_recipient_ata,
            quote_token_program: self.quote_token_program,
            paused: self.paused,
            routable: self.routable,
        }
    }
}

impl XDSwapAmm {
    fn derive_global_config(program_id: Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"global_config"], &program_id).0
    }

    fn is_buy_pair(
        input_mint: Pubkey,
        output_mint: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
    ) -> bool {
        input_mint == quote_mint && output_mint == base_mint
    }

    fn is_sell_pair(
        input_mint: Pubkey,
        output_mint: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
    ) -> bool {
        input_mint == base_mint && output_mint == quote_mint
    }
}

impl Amm for XDSwapAmm {
    fn from_keyed_account(keyed_account: &KeyedAccount, _amm_context: &AmmContext) -> Result<Self> {
        let pool = XDSwapPool::decode(&keyed_account.account.data)?;
        let program_id = keyed_account.account.owner;
        let global_config_key = Self::derive_global_config(program_id);

        Ok(Self {
            program_id,
            key: keyed_account.key,
            index: pool.index,
            creator: pool.creator,
            base_mint: pool.base_mint,
            quote_mint: pool.quote_mint,
            base_vault: pool.base_vault,
            quote_vault: pool.quote_vault,
            base_reserves: pool.base_reserves,
            quote_reserves: pool.quote_reserves,
            is_active: pool.is_active,
            bump: pool.bump,
            creator_fee_multiplier_bps: pool.creator_fee_multiplier_bps,
            global_config_key,
            protocol_fee_recipient: Pubkey::default(),
            protocol_fee_recipient_ata: Pubkey::default(),
            quote_token_program: Pubkey::default(),
            paused: true,
            routable: false,
        })
    }

    fn label(&self) -> String {
        "XDSwap".into()
    }

    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        vec![self.base_mint, self.quote_mint]
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        let mut accounts = vec![self.key, self.global_config_key, self.quote_mint];

        if self.protocol_fee_recipient != Pubkey::default() {
            if self.quote_token_program == TOKEN_PROGRAM_ID
                || self.quote_token_program == TOKEN_2022_PROGRAM_ID
            {
                accounts.push(get_associated_token_address(
                    &self.protocol_fee_recipient,
                    &self.quote_mint,
                    &self.quote_token_program,
                ));
            } else {
                // Before quote token program is resolved, request both candidate ATAs.
                let token_ata = get_associated_token_address(
                    &self.protocol_fee_recipient,
                    &self.quote_mint,
                    &TOKEN_PROGRAM_ID,
                );
                let token_2022_ata = get_associated_token_address(
                    &self.protocol_fee_recipient,
                    &self.quote_mint,
                    &TOKEN_2022_PROGRAM_ID,
                );
                accounts.push(token_ata);
                if token_2022_ata != token_ata {
                    accounts.push(token_2022_ata);
                }
            }
        }

        accounts
    }

    fn has_dynamic_accounts(&self) -> bool {
        true
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        // Re-decode Pool
        let pool_data = try_get_account_data(account_map, &self.key)?;
        let pool = XDSwapPool::decode(pool_data)?;

        self.index = pool.index;
        self.creator = pool.creator;
        self.base_mint = pool.base_mint;
        self.quote_mint = pool.quote_mint;
        self.base_vault = pool.base_vault;
        self.quote_vault = pool.quote_vault;
        self.base_reserves = pool.base_reserves;
        self.quote_reserves = pool.quote_reserves;
        self.is_active = pool.is_active;
        self.bump = pool.bump;
        self.creator_fee_multiplier_bps = pool.creator_fee_multiplier_bps;

        // Re-decode GlobalConfig
        let config_data = try_get_account_data(account_map, &self.global_config_key)?;
        let config = XDSwapGlobalConfig::decode(config_data)?;

        self.protocol_fee_recipient = config.protocol_fee_recipients[0];
        self.paused = config.paused;

        // Resolve quote_mint token program from its account owner
        let quote_mint_account = account_map
            .get(&self.quote_mint)
            .ok_or_else(|| anyhow::anyhow!("quote_mint account not found: {}", self.quote_mint))?;

        let owner = quote_mint_account.owner;

        if owner == TOKEN_PROGRAM_ID {
            self.quote_token_program = TOKEN_PROGRAM_ID;
        } else if owner == TOKEN_2022_PROGRAM_ID {
            self.quote_token_program = TOKEN_2022_PROGRAM_ID;
        } else {
            self.routable = false;
            bail!(
                "Unsupported quote mint owner for XDSwapAmm: quote_mint={} owner={}",
                self.quote_mint,
                owner
            );
        }

        // Derive protocol fee recipient ATA
        self.protocol_fee_recipient_ata = get_associated_token_address(
            &self.protocol_fee_recipient,
            &self.quote_mint,
            &self.quote_token_program,
        );

        // Fee ATA must exist and be loaded in AccountMap to be routable.
        if !account_map.contains_key(&self.protocol_fee_recipient_ata) {
            self.routable = false;
            return Ok(());
        }

        self.routable = true;
        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        if !self.routable {
            bail!("XDSwapAmm is not routable: invalid quote token program or fee ATA unresolvable");
        }

        let is_buy = Self::is_buy_pair(
            quote_params.input_mint,
            quote_params.output_mint,
            self.base_mint,
            self.quote_mint,
        );
        let is_sell = Self::is_sell_pair(
            quote_params.input_mint,
            quote_params.output_mint,
            self.base_mint,
            self.quote_mint,
        );

        if !is_buy && !is_sell {
            bail!(
                "Invalid mint pair for XDSwapAmm: input={} output={}",
                quote_params.input_mint,
                quote_params.output_mint
            );
        }

        let (base_creator_fee_bps, protocol_fee_bps, lp_fee_bps) =
            fee_math::get_dynamic_fees(self.base_reserves, self.quote_reserves);

        let eff_creator_bps = xd_math::effective_creator_fee_bps(
            base_creator_fee_bps,
            self.creator_fee_multiplier_bps,
        );

        match (is_buy, quote_params.swap_mode) {
            (true, SwapMode::ExactIn) => {
                let result = xd_math::calculate_buy_exact_in(
                    quote_params.amount,
                    self.base_reserves,
                    self.quote_reserves,
                    lp_fee_bps,
                    protocol_fee_bps,
                    eff_creator_bps,
                )
                .ok_or_else(|| anyhow::anyhow!("BuyExactIn calculation overflow"))?;

                let fee_pct = if quote_params.amount > 0 {
                    Decimal::from(result.total_fee) / Decimal::from(quote_params.amount)
                } else {
                    Decimal::ZERO
                };

                Ok(Quote {
                    in_amount: quote_params.amount,
                    out_amount: result.base_out,
                    fee_amount: result.total_fee,
                    fee_mint: self.quote_mint,
                    fee_pct,
                })
            }

            (true, SwapMode::ExactOut) => {
                let result = xd_math::calculate_buy_exact_out(
                    quote_params.amount,
                    self.base_reserves,
                    self.quote_reserves,
                    lp_fee_bps,
                    protocol_fee_bps,
                    eff_creator_bps,
                )
                .ok_or_else(|| anyhow::anyhow!("BuyExactOut calculation overflow"))?;

                let fee_pct = if result.total_quote_in > 0 {
                    Decimal::from(result.total_fee) / Decimal::from(result.total_quote_in)
                } else {
                    Decimal::ZERO
                };

                Ok(Quote {
                    in_amount: result.total_quote_in,
                    out_amount: quote_params.amount,
                    fee_amount: result.total_fee,
                    fee_mint: self.quote_mint,
                    fee_pct,
                })
            }

            (false, SwapMode::ExactIn) => {
                let result = xd_math::calculate_sell(
                    quote_params.amount,
                    self.base_reserves,
                    self.quote_reserves,
                    lp_fee_bps,
                    protocol_fee_bps,
                    eff_creator_bps,
                )
                .ok_or_else(|| anyhow::anyhow!("Sell calculation overflow"))?;

                let quote_out_raw = result.quote_out + result.total_fee;
                let fee_pct = if quote_out_raw > 0 {
                    Decimal::from(result.total_fee) / Decimal::from(quote_out_raw)
                } else {
                    Decimal::ZERO
                };

                Ok(Quote {
                    in_amount: quote_params.amount,
                    out_amount: result.quote_out,
                    fee_amount: result.total_fee,
                    fee_mint: self.quote_mint,
                    fee_pct,
                })
            }

            (false, SwapMode::ExactOut) => {
                let result = xd_math::calculate_sell_exact_out(
                    quote_params.amount,
                    self.base_reserves,
                    self.quote_reserves,
                    lp_fee_bps,
                    protocol_fee_bps,
                    eff_creator_bps,
                )
                .ok_or_else(|| anyhow::anyhow!("SellExactOut calculation overflow"))?;

                let fee_pct = if (result.total_fee + quote_params.amount) > 0 {
                    Decimal::from(result.total_fee)
                        / Decimal::from(result.total_fee + quote_params.amount)
                } else {
                    Decimal::ZERO
                };

                Ok(Quote {
                    in_amount: result.base_amount_in,
                    out_amount: quote_params.amount,
                    fee_amount: result.total_fee,
                    fee_mint: self.quote_mint,
                    fee_pct,
                })
            }
        }
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        if !self.routable {
            bail!("XDSwapAmm is not routable: invalid quote token program or fee ATA unresolvable");
        }

        let is_buy = Self::is_buy_pair(
            swap_params.source_mint,
            swap_params.destination_mint,
            self.base_mint,
            self.quote_mint,
        );
        let is_sell = Self::is_sell_pair(
            swap_params.source_mint,
            swap_params.destination_mint,
            self.base_mint,
            self.quote_mint,
        );

        if !is_buy && !is_sell {
            bail!(
                "Invalid mint pair for XDSwapAmm: source={} destination={}",
                swap_params.source_mint,
                swap_params.destination_mint
            );
        }

        let account_metas = if is_buy {
            vec![
                AccountMeta::new(swap_params.token_transfer_authority, true),
                AccountMeta::new_readonly(self.global_config_key, false),
                AccountMeta::new(self.key, false),
                AccountMeta::new_readonly(self.base_mint, false),
                AccountMeta::new_readonly(self.quote_mint, false),
                AccountMeta::new(self.base_vault, false),
                AccountMeta::new(self.quote_vault, false),
                AccountMeta::new(swap_params.destination_token_account, false),
                AccountMeta::new(swap_params.source_token_account, false),
                AccountMeta::new(self.protocol_fee_recipient_ata, false),
                AccountMeta::new_readonly(self.quote_token_program, false),
                AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
                AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            ]
        } else {
            vec![
                AccountMeta::new(swap_params.token_transfer_authority, true),
                AccountMeta::new_readonly(self.global_config_key, false),
                AccountMeta::new(self.key, false),
                AccountMeta::new_readonly(self.base_mint, false),
                AccountMeta::new_readonly(self.quote_mint, false),
                AccountMeta::new(self.base_vault, false),
                AccountMeta::new(self.quote_vault, false),
                AccountMeta::new(swap_params.source_token_account, false),
                AccountMeta::new(swap_params.destination_token_account, false),
                AccountMeta::new(self.protocol_fee_recipient_ata, false),
                AccountMeta::new_readonly(self.quote_token_program, false),
                AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
                AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            ]
        };

        // Prepend program ID (Jupiter convention)
        let mut metas = vec![AccountMeta::new_readonly(self.program_id, false)];
        metas.extend(account_metas);

        // Uses PumpSwap variants (same instruction layout).
        let swap = if is_buy {
            Swap::PumpSwapBuy
        } else {
            Swap::PumpSwapSell
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
        true
    }

    fn is_active(&self) -> bool {
        self.is_active && !self.paused
    }

    fn get_accounts_len(&self) -> usize {
        15
    }
}
