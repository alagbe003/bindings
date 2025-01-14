use crate::{helper::get_discount, msg::ReplyType};

use super::*;
use cosmwasm_std::{
    coin, to_json_binary, OverflowError, OverflowOperation, SignedDecimal, SignedDecimal256,
    StdError, StdResult, SubMsg,
};
use cw_utils;
use elys_bindings::query_resp::{Entry, QueryGetEntryResponse};
use PerpetualOrderType::*;

pub fn create_perpetual_order(
    env: Env,
    info: MessageInfo,
    deps: DepsMut<ElysQuery>,
    position: Option<PerpetualPosition>,
    leverage: Option<SignedDecimal>,
    trading_asset: Option<String>,
    take_profit_price: Option<SignedDecimal256>,
    order_type: PerpetualOrderType,
    trigger_price: Option<OrderPrice>,
    position_id: Option<u64>,
) -> Result<Response<ElysMsg>, ContractError> {
    check_order_type(
        &position,
        &leverage,
        &trading_asset,
        &order_type,
        &trigger_price,
        &position_id,
    )?;

    if order_type == LimitOpen || order_type == MarketOpen {
        create_perpetual_open_order(
            info,
            deps,
            order_type,
            position.unwrap(),
            trading_asset.unwrap(),
            leverage.unwrap(),
            take_profit_price,
            trigger_price,
            env.contract.address.as_str(),
        )
    } else {
        create_perpetual_close_order(
            env.contract.address.as_str(),
            info,
            deps,
            order_type,
            position_id.unwrap(),
            trigger_price,
        )
    }
}

fn check_order_type(
    position: &Option<PerpetualPosition>,
    leverage: &Option<SignedDecimal>,
    trading_asset: &Option<String>,
    order_type: &PerpetualOrderType,
    trigger_price: &Option<OrderPrice>,
    position_id: &Option<u64>,
) -> StdResult<()> {
    let mut not_found: Vec<&str> = vec![];

    if order_type != &MarketOpen && order_type != &MarketClose && trigger_price.is_none() {
        not_found.push("trigger price");
    }

    if (order_type == &LimitClose || order_type == &MarketClose || order_type == &StopLoss)
        && position_id.is_none()
    {
        not_found.push("position id");
    }

    if order_type == &LimitOpen || order_type == &MarketOpen {
        if position.is_none() {
            not_found.push("position");
        }
        if leverage.is_none() {
            not_found.push("leverage");
        }
        if trading_asset.is_none() {
            not_found.push("borrow asset");
        }
    }

    if not_found.is_empty() {
        Ok(())
    } else {
        let missing_fields = not_found.join(", ");
        Err(StdError::generic_err(format!(
            "Missing fields: {}",
            missing_fields
        )))
    }
}

fn create_perpetual_open_order(
    info: MessageInfo,
    deps: DepsMut<ElysQuery>,
    order_type: PerpetualOrderType,
    position: PerpetualPosition,
    trading_asset: String,
    leverage: SignedDecimal,
    take_profit_price: Option<SignedDecimal256>,
    trigger_price: Option<OrderPrice>,
    creator: &str,
) -> Result<Response<ElysMsg>, ContractError> {
    let collateral = cw_utils::one_coin(&info)?;

    let orders: Vec<PerpetualOrder> = PERPETUAL_ORDER
        .prefix_range(deps.storage, None, None, Order::Ascending)
        .filter_map(|res| res.ok().map(|r| r.1))
        .collect();

    if position == PerpetualPosition::Unspecified {
        return Err(
            StdError::generic_err("perpetual position cannot be set at: Unspecified").into(),
        );
    }

    let querier = ElysQuerier::new(&deps.querier);
    let QueryGetEntryResponse {
        entry: Entry {
            denom: usdc_denom, ..
        },
    } = querier.get_asset_profile("uusdc".to_string())?;

    let open_estimation = querier.perpetual_open_estimation(
        position.clone(),
        leverage.clone(),
        &trading_asset,
        collateral.clone(),
        take_profit_price.clone(),
        get_discount(&deps.as_ref(), info.sender.to_string())?,
    )?;

    if !open_estimation.valid_collateral {
        return Err(StdError::generic_err("not valid collateral").into());
    }

    if let Some(price) = &trigger_price {
        if price.rate.is_zero() {
            return Err(StdError::generic_err("trigger_price: The rate cannot be zero").into());
        }

        if price.base_denom != usdc_denom {
            return Err(StdError::generic_err(
                "trigger_price: The base denom should be the usdc denom",
            )
            .into());
        }

        if price.quote_denom != trading_asset {
            return Err(StdError::generic_err(
                "trigger_price: The quote denom should be the trading asset denom",
            )
            .into());
        }
    }

    let order = PerpetualOrder::new_open(
        &info.sender,
        &position,
        &order_type,
        &collateral,
        &trading_asset,
        &leverage,
        &take_profit_price,
        &trigger_price,
        &orders,
    )?;

    let order_id = order.order_id;

    PERPETUAL_ORDER.save(deps.storage, order_id, &order)?;
    if order.order_type != PerpetualOrderType::MarketOpen {
        PENDING_PERPETUAL_ORDER.save(deps.storage, order_id, &order)?;
    }

    let resp = Response::new().add_event(
        Event::new("create_perpetual_open_order")
            .add_attribute("perpetual_order_id", order_id.to_string()),
    );

    if order_type != MarketOpen {
        return Ok(resp);
    }

    let msg = ElysMsg::perpetual_open_position(
        creator,
        collateral,
        trading_asset,
        position,
        leverage,
        take_profit_price,
        info.sender,
    );

    let reply_info_max_id = MAX_REPLY_ID.load(deps.storage)?;

    let reply_id = match reply_info_max_id.checked_add(1) {
        Some(id) => id,
        None => {
            return Err(StdError::overflow(OverflowError::new(
                OverflowOperation::Add,
                "reply_info_max_id",
                "increment one",
            ))
            .into())
        }
    };
    MAX_REPLY_ID.save(deps.storage, &reply_id)?;

    let reply_info = ReplyInfo {
        id: reply_id,
        reply_type: ReplyType::PerpetualBrokerMarketOpen,
        data: Some(to_json_binary(&order_id)?),
    };

    REPLY_INFO.save(deps.storage, reply_id, &reply_info)?;

    let sub_msg = SubMsg::reply_always(msg, reply_id);

    Ok(resp.add_submessage(sub_msg))
}

fn create_perpetual_close_order(
    creator: &str,
    info: MessageInfo,
    deps: DepsMut<ElysQuery>,
    order_type: PerpetualOrderType,
    position_id: u64,
    trigger_price: Option<OrderPrice>,
) -> Result<Response<ElysMsg>, ContractError> {
    cw_utils::nonpayable(&info)?;

    let querier = ElysQuerier::new(&deps.querier);

    let mtp_resp = querier.mtp(info.sender.to_string(), position_id)?;

    let mtp = if let Some(mtp) = mtp_resp.mtp {
        mtp
    } else {
        return Err(StdError::not_found("perpetual trading position").into());
    };

    let orders: Vec<PerpetualOrder> = PERPETUAL_ORDER
        .prefix_range(deps.storage, None, None, Order::Ascending)
        .filter_map(|res| res.ok().map(|r| r.1))
        .collect();

    let QueryGetEntryResponse {
        entry: Entry {
            denom: usdc_denom, ..
        },
    } = querier.get_asset_profile("uusdc".to_string())?;

    if let Some(price) = &trigger_price {
        if price.rate.is_zero() {
            return Err(StdError::generic_err("trigger_price: The rate cannot be zero").into());
        }

        if price.base_denom != usdc_denom {
            return Err(StdError::generic_err(
                "trigger_price: The base denom should be the usdc denom",
            )
            .into());
        }

        if price.quote_denom != mtp.trading_asset {
            return Err(StdError::generic_err(
                "trigger_price: The quote denom should be the trading asset denom",
            )
            .into());
        }
    }

    if let Some(mut order) = orders
        .iter()
        .find(|order| {
            order.position_id == Some(position_id)
                && order.status == Status::Pending
                && order_type == order.order_type
        })
        .cloned()
    {
        order.trigger_price = trigger_price;
        PERPETUAL_ORDER.save(deps.storage, order.order_id, &order)?;

        if order.order_type != PerpetualOrderType::MarketClose {
            PENDING_PERPETUAL_ORDER.save(deps.storage, order.order_id, &order)?;
        }

        let resp = Response::new().add_event(
            Event::new("create_perpetual_close_order")
                .add_attribute("perpetual_order_id", order.order_id.to_string()),
        );

        return Ok(resp);
    };

    let order = PerpetualOrder::new_close(
        &info.sender,
        mtp.position,
        &order_type,
        &coin(mtp.collateral.i128() as u128, &mtp.collateral_asset),
        &mtp.trading_asset,
        &mtp.leverage,
        position_id,
        &trigger_price,
        &Some(mtp.take_profit_price),
        &orders,
    )?;

    let order_id = order.order_id;

    PERPETUAL_ORDER.save(deps.storage, order_id, &order)?;
    if order.order_type != PerpetualOrderType::MarketClose {
        PENDING_PERPETUAL_ORDER.save(deps.storage, order_id, &order)?;
    }

    let resp = Response::new().add_event(
        Event::new("create_perpetual_close_order")
            .add_attribute("perpetual_order_id", order_id.to_string()),
    );

    if order_type != MarketClose {
        return Ok(resp);
    }

    let msg =
        ElysMsg::perpetual_close_position(creator, position_id, mtp.custody.i128(), &info.sender);

    let reply_info_max_id = MAX_REPLY_ID.load(deps.storage)?;

    let reply_id = match reply_info_max_id.checked_add(1) {
        Some(id) => id,
        None => {
            return Err(StdError::overflow(OverflowError::new(
                OverflowOperation::Add,
                "reply_info_max_id",
                "increment one",
            ))
            .into())
        }
    };
    MAX_REPLY_ID.save(deps.storage, &reply_id)?;

    let reply_info = ReplyInfo {
        id: reply_id,
        reply_type: ReplyType::PerpetualBrokerMarketClose,
        data: Some(to_json_binary(&order_id)?),
    };

    REPLY_INFO.save(deps.storage, reply_id, &reply_info)?;

    let sub_msg = SubMsg::reply_always(msg, reply_id);

    Ok(resp.add_submessage(sub_msg))
}
