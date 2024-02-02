use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Coin, Decimal, StdError, StdResult};
use elys_bindings::{query_resp::OracleAssetInfoResponse, types::OracleAssetInfo, ElysQuerier};

#[cw_serde]
pub struct CoinValue {
    pub denom: String,
    pub amount_token: Decimal,
    pub price: Decimal,
    pub amount_usdc: Decimal,
}

impl CoinValue {
    pub fn new(denom: String, amount_token: Decimal, price: Decimal, amount_usdc: Decimal) -> Self {
        Self {
            denom,
            amount_token,
            price,
            amount_usdc,
        }
    }
    pub fn from_coin(
        balance: &Coin,
        querier: &ElysQuerier<'_>,
        usdc_denom: &String,
    ) -> StdResult<Self> {
        let OracleAssetInfoResponse { asset_info } = match querier.asset_info(balance.denom.clone())
        {
            Ok(res) => res,
            Err(_) => OracleAssetInfoResponse {
                asset_info: OracleAssetInfo {
                    denom: balance.denom.clone(),
                    display: balance.denom.clone(),
                    band_ticker: balance.denom.clone(),
                    elys_ticker: balance.denom.clone(),
                    decimal: 6,
                },
            },
        };
        let decimal_point_token = asset_info.decimal;

        if &balance.denom == usdc_denom {
            let amount = Decimal::from_atomics(balance.amount, decimal_point_token as u32)
                .map_err(|e| {
                    StdError::generic_err(format!("failed to convert amount to Decimal: {}", e))
                })?;
            return Ok(Self {
                denom: balance.denom.clone(),
                amount_usdc: amount.clone(),
                price: Decimal::one(),
                amount_token: amount,
            });
        }

        let price = querier
            .get_amm_price_by_denom(coin(1, balance.denom.clone()), Decimal::zero())
            .map_err(|e| {
                StdError::generic_err(format!("failed to get_amm_price_by_denom: {}", e))
            })?;

        let decimal_point_usdc = asset_info.decimal;

        let amount_token = Decimal::from_atomics(balance.amount, decimal_point_token as u32)
            .map_err(|e| {
                StdError::generic_err(format!("failed to convert amount to Decimal: {}", e))
            })?;

        let amount_usdc = price
            .clone()
            .checked_mul(
                Decimal::from_atomics(balance.amount, decimal_point_usdc as u32).map_err(|e| {
                    StdError::generic_err(format!(
                        "failed to convert amount_usdc_base to Decimal: {}",
                        e
                    ))
                })?,
            )
            .map_err(|e| {
                StdError::generic_err(format!(
                    "failed to convert amount_usdc_base to Decimal: {}",
                    e
                ))
            })?;

        Ok(Self {
            denom: balance.denom.clone(),
            amount_token,
            price,
            amount_usdc,
        })
    }
}
