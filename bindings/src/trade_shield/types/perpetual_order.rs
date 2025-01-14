use crate::types::PerpetualPosition;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, OverflowError, SignedDecimal, SignedDecimal256, StdError, StdResult};

use super::{OrderPrice, PerpetualOrderType, Status};

#[cw_serde]
pub struct PerpetualOrder {
    pub order_id: u64,
    pub owner: String,
    pub order_type: PerpetualOrderType,
    pub position: PerpetualPosition,
    pub trigger_price: Option<OrderPrice>,
    pub collateral: Coin,
    pub trading_asset: String,
    pub leverage: SignedDecimal,
    pub take_profit_price: Option<SignedDecimal256>,
    pub position_id: Option<u64>,
    pub status: Status,
}

impl PerpetualOrder {
    pub fn new_open(
        owner: impl Into<String>,
        position: &PerpetualPosition,
        order_type: &PerpetualOrderType,
        collateral: &Coin,
        trading_asset: impl Into<String>,
        leverage: &SignedDecimal,
        take_profit_price: &Option<SignedDecimal256>,
        trigger_price: &Option<OrderPrice>,
        order_vec: &Vec<PerpetualOrder>,
    ) -> StdResult<Self> {
        let status = Status::Pending;

        let order_id = get_new_id(&order_vec)?;

        let order = Self {
            order_id,
            owner: owner.into(),
            position: position.to_owned(),
            collateral: collateral.to_owned(),
            trading_asset: trading_asset.into(),
            leverage: leverage.to_owned(),
            take_profit_price: take_profit_price.to_owned(),
            order_type: order_type.to_owned(),
            trigger_price: trigger_price.to_owned(),
            status,
            position_id: None,
        };

        return Ok(order);
    }
    pub fn new_close(
        owner: impl Into<String>,
        position: i32,
        order_type: &PerpetualOrderType,
        collateral: &Coin,
        trading_asset: impl Into<String>,
        leverage: &SignedDecimal,
        position_id: u64,
        trigger_price: &Option<OrderPrice>,
        take_profit_price: &Option<SignedDecimal256>,
        order_vec: &Vec<PerpetualOrder>,
    ) -> StdResult<Self> {
        let order_id: u64 = get_new_id(&order_vec)?;

        let status = Status::Pending;

        let position = PerpetualPosition::try_from_i32(position)?;

        let order: PerpetualOrder = Self {
            order_id,
            status,
            order_type: order_type.to_owned(),
            position,
            owner: owner.into(),
            trigger_price: trigger_price.to_owned(),
            collateral: collateral.to_owned(),
            trading_asset: trading_asset.into(),
            position_id: Some(position_id),
            leverage: leverage.to_owned(),
            take_profit_price: take_profit_price.to_owned(),
        };

        Ok(order)
    }
}

fn get_new_id(orders: &[PerpetualOrder]) -> StdResult<u64> {
    match orders.iter().max_by_key(|s| s.order_id) {
        Some(order) => match order.order_id.checked_add(1) {
            Some(id) => Ok(id),
            None => Err(StdError::overflow(OverflowError::new(
                cosmwasm_std::OverflowOperation::Add,
                "perpetual_order_max_id",
                "increment one",
            ))),
        },
        None => Ok(0),
    }
}
