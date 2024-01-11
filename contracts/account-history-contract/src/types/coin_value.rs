use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, StdError, StdResult};
use elys_bindings::{
    query_resp::{AmmSwapEstimationByDenomResponse, OracleAssetInfoResponse},
    types::OracleAssetInfo,
    ElysQuerier,
};

#[cw_serde]
pub struct CoinValue {
    denom: String,
    amount: Decimal,
    price: Decimal,
    value: Decimal,
}

impl CoinValue {
    pub fn new(denom: String, amount: Decimal, price: Decimal, value: Decimal) -> Self {
        Self {
            denom,
            amount,
            price,
            value,
        }
    }
    pub fn from_coin(
        coin: &Coin,
        querier: &ElysQuerier<'_>,
        value_denom: &String,
    ) -> StdResult<Self> {
        let AmmSwapEstimationByDenomResponse {
            spot_price: price,
            amount: whole_value,
            ..
        } = querier.amm_swap_estimation_by_denom(
            &coin,
            &coin.denom,
            value_denom,
            &Decimal::zero(),
        )?;

        let OracleAssetInfoResponse {
            asset_info:
                OracleAssetInfo {
                    decimal: decimal_point_value,
                    ..
                },
        } = querier.asset_info(value_denom.to_owned())?;

        let OracleAssetInfoResponse {
            asset_info:
                OracleAssetInfo {
                    decimal: decimal_point_coin,
                    ..
                },
        } = querier.asset_info(coin.denom.clone())?;

        let amount = Decimal::from_atomics(coin.amount, decimal_point_coin as u32)
            .map_err(|err| StdError::generic_err(err.to_string()))?;

        let value = Decimal::from_atomics(whole_value.amount, decimal_point_value as u32)
            .map_err(|err| StdError::generic_err(err.to_string()))?;

        Ok(Self {
            denom: coin.denom,
            amount,
            price,
            value,
        })
    }
}
