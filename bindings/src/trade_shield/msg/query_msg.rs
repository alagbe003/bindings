#[allow(unused_imports)]
use super::query_resp::*;
use crate::trade_shield::types::{MarginOrderType, SpotOrderType, Status};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, SignedDecimal, SignedDecimal256};
#[allow(unused_imports)]
use crate::query_resp::*;
use crate::types::{MarginPosition, PageRequest};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetSpotOrderResp)]
    GetSpotOrder { order_id: u64 },
    #[returns(GetAllPricesResponse)]
    GetAllPrices { limit: u64 },
    #[returns(OracleAssetInfoResponse)]
    AssetInfo { denom: String },
    #[returns(GetMarginOrderResp)]
    GetMarginOrder { id: u64 },
    #[returns(GetSpotOrdersResp)]
    GetSpotOrders {
        pagination: Option<PageRequest>,
        order_owner: Option<String>,
        order_type: Option<SpotOrderType>,
        order_status: Option<Status>,
    },
    #[returns(GetMarginOrdersResp)]
    GetMarginOrders {
        pagination: Option<PageRequest>,
        order_owner: Option<String>,
        order_type: Option<MarginOrderType>,
        order_status: Option<Status>,
    },
    #[returns(AmmSwapEstimationByDenomResponse)]
    SwapEstimationByDenom {
        amount: Coin,
        denom_in: String,
        denom_out: String,
        user_address: Option<String>,
    },
    #[returns(MarginMtpResponse)]
    GetMarginPosition { id: u64, address: String },
    #[returns(MarginQueryPositionsResponse)]
    GetMarginPositions { pagination: PageRequest },
    #[returns(MarginOpenEstimationResponse)]
    MarginOpenEstimation {
        position: MarginPosition,
        leverage: SignedDecimal,
        trading_asset: String,
        collateral: Coin,
        take_profit_price: SignedDecimal256,
        user_address: Option<String>,
    },
    #[returns(MarginGetPositionsForAddressResponse)]
    MarginGetPositionsForAddress {
        address: String,
        pagination: PageRequest,
    },
}