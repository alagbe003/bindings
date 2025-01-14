use crate::{helper::get_discount, msg::ReplyType};
use cosmwasm_std::{
    to_json_binary, Decimal, Int128, OverflowError, StdError, StdResult, Storage, SubMsg,
};
use elys_bindings::query_resp::{AmmSwapEstimationByDenomResponse, Entry, QueryGetEntryResponse};

use super::*;

pub fn process_orders(
    deps: DepsMut<ElysQuery>,
    env: Env,
) -> Result<Response<ElysMsg>, ContractError> {
    let spot_orders: Vec<SpotOrder> = PENDING_SPOT_ORDER
        .prefix_range(deps.storage, None, None, Order::Ascending)
        .filter_map(|res| res.ok().map(|r| r.1))
        .collect();

    let perpetual_orders: Vec<PerpetualOrder> = PENDING_PERPETUAL_ORDER
        .prefix_range(deps.storage, None, None, Order::Ascending)
        .filter_map(|res| res.ok().map(|r| r.1))
        .collect();

    let mut reply_info_id = MAX_REPLY_ID.load(deps.storage)?;

    let querier = ElysQuerier::new(&deps.querier);
    let mut submsgs: Vec<SubMsg<ElysMsg>> = vec![];
    let mut bank_msgs: Vec<BankMsg> = vec![];

    let QueryGetEntryResponse {
        entry: Entry {
            denom: usdc_denom, ..
        },
    } = querier.get_asset_profile("uusdc".to_string())?;

    for spot_order in spot_orders.iter() {
        if spot_order.order_price.base_denom != spot_order.order_amount.denom
            || spot_order.order_price.quote_denom != spot_order.order_target_denom
        {
            let mut order = spot_order.to_owned();
            order.status = Status::Canceled;
            bank_msgs.push(BankMsg::Send {
                to_address: order.owner_address.to_string(),
                amount: vec![order.order_amount.clone()],
            });
            PENDING_SPOT_ORDER.remove(deps.storage, order.order_id);
            SPOT_ORDER.save(deps.storage, order.order_id, &order)?;
            continue;
        }

        let discount = get_discount(&deps.as_ref(), spot_order.owner_address.to_string())?;

        let amm_swap_estimation = match querier.amm_swap_estimation_by_denom(
            &spot_order.order_amount,
            &spot_order.order_amount.denom,
            &spot_order.order_target_denom,
            &discount,
        ) {
            Ok(market_price) => market_price,
            Err(_) => {
                let mut order = spot_order.to_owned();
                order.status = Status::Canceled;
                bank_msgs.push(BankMsg::Send {
                    to_address: order.owner_address.to_string(),
                    amount: vec![order.order_amount.clone()],
                });
                PENDING_SPOT_ORDER.remove(deps.storage, order.order_id);
                SPOT_ORDER.save(deps.storage, order.order_id, &order)?;
                continue;
            }
        };

        let market_price = match querier.get_asset_price_from_denom_in_to_denom_out(
            &spot_order.order_amount.denom,
            &spot_order.order_target_denom,
        ) {
            Ok(market_price) => market_price,
            Err(_) => {
                let mut order = spot_order.to_owned();
                order.status = Status::Canceled;
                bank_msgs.push(BankMsg::Send {
                    to_address: order.owner_address.to_string(),
                    amount: vec![order.order_amount.clone()],
                });
                PENDING_SPOT_ORDER.remove(deps.storage, order.order_id);
                SPOT_ORDER.save(deps.storage, order.order_id, &order)?;
                continue;
            }
        };

        if check_spot_order(&spot_order, market_price) {
            process_spot_order(
                spot_order,
                &mut submsgs,
                env.contract.address.as_str(),
                &mut reply_info_id,
                amm_swap_estimation,
                deps.storage,
                discount,
            )?;
        }
    }

    for perpetual_order in perpetual_orders.iter() {
        let mut order = perpetual_order.to_owned();

        if perpetual_order.trigger_price.as_ref().unwrap().base_denom != usdc_denom
            || perpetual_order.trigger_price.as_ref().unwrap().quote_denom
                != perpetual_order.trading_asset
        {
            let mut order = perpetual_order.to_owned();
            order.status = Status::Canceled;
            if perpetual_order.order_type == PerpetualOrderType::LimitOpen {
                bank_msgs.push(BankMsg::Send {
                    to_address: order.owner.clone(),
                    amount: vec![order.collateral.clone()],
                })
            }
            PENDING_PERPETUAL_ORDER.remove(deps.storage, order.order_id);
            PERPETUAL_ORDER.save(deps.storage, order.order_id, &order)?;
            continue;
        }

        let market_price = match querier.get_asset_price_from_denom_in_to_denom_out(
            &perpetual_order.collateral.denom,
            &perpetual_order.trading_asset,
        ) {
            Ok(market_price) => market_price,
            Err(_) => {
                order.status = Status::Canceled;
                PENDING_PERPETUAL_ORDER.remove(deps.storage, order.order_id);
                PERPETUAL_ORDER.save(deps.storage, order.order_id, &order)?;
                if order.order_type == PerpetualOrderType::LimitOpen {
                    bank_msgs.push(BankMsg::Send {
                        to_address: order.owner.to_string(),
                        amount: vec![order.collateral.clone()],
                    });
                }
                continue;
            }
        };

        if order.order_type != PerpetualOrderType::LimitOpen {
            match querier.mtp(order.owner.clone(), order.position_id.clone().unwrap()) {
                Ok(mtp) => match mtp.mtp {
                    Some(_) => {}
                    None => {
                        order.status = Status::Canceled;
                        PENDING_PERPETUAL_ORDER.remove(deps.storage, order.order_id);
                        PERPETUAL_ORDER.save(deps.storage, order.order_id, &order)?;
                        continue;
                    }
                },
                Err(_) => {
                    order.status = Status::Canceled;
                    PENDING_PERPETUAL_ORDER.remove(deps.storage, order.order_id);
                    PERPETUAL_ORDER.save(deps.storage, order.order_id, &order)?;
                    continue;
                }
            };
        }

        if check_perpetual_order(&perpetual_order, market_price) {
            process_perpetual_order(
                perpetual_order,
                &mut submsgs,
                &mut reply_info_id,
                deps.storage,
                &querier,
                env.contract.address.as_str(),
            )?;
        }
    }

    MAX_REPLY_ID.save(deps.storage, &reply_info_id)?;

    let resp = Response::new().add_submessages(submsgs);

    Ok(resp)
}

fn process_perpetual_order(
    order: &PerpetualOrder,
    submsgs: &mut Vec<SubMsg<ElysMsg>>,
    reply_info_id: &mut u64,
    storage: &mut dyn Storage,
    querier: &ElysQuerier<'_>,
    creator: &str,
) -> StdResult<()> {
    let (msg, reply_type) = if order.order_type == PerpetualOrderType::LimitOpen {
        (
            ElysMsg::perpetual_open_position(
                creator,
                order.collateral.clone(),
                &order.trading_asset,
                order.position.clone(),
                order.leverage.clone(),
                order.take_profit_price.clone(),
                &order.owner,
            ),
            ReplyType::PerpetualBrokerOpen,
        )
    } else {
        let mtp = match querier
            .mtp(order.owner.clone(), order.position_id.unwrap())?
            .mtp
        {
            Some(mtp) => mtp,
            None => {
                let mut order = order.to_owned();
                order.status = Status::Canceled;
                PENDING_PERPETUAL_ORDER.remove(storage, order.order_id);
                PERPETUAL_ORDER.save(storage, order.order_id, &order)?;
                return Ok(());
            }
        };

        let amount = mtp.custody.i128();
        (
            ElysMsg::perpetual_close_position(
                creator,
                order.position_id.unwrap(),
                amount,
                &order.owner,
            ),
            ReplyType::PerpetualBrokerClose,
        )
    };

    *reply_info_id = match reply_info_id.checked_add(1) {
        Some(id) => id,
        None => {
            return Err(StdError::overflow(OverflowError::new(
                cosmwasm_std::OverflowOperation::Add,
                "reply_info_max_id",
                "increment one",
            ))
            .into())
        }
    };

    let reply_info = ReplyInfo {
        id: *reply_info_id,
        reply_type,
        data: Some(to_json_binary(&order.order_id)?),
    };
    submsgs.push(SubMsg::reply_always(msg, *reply_info_id));

    REPLY_INFO.save(storage, *reply_info_id, &reply_info)?;

    Ok(())
}

fn check_perpetual_order(order: &PerpetualOrder, market_price: Decimal) -> bool {
    if order.order_type == PerpetualOrderType::MarketClose
        || order.order_type == PerpetualOrderType::MarketOpen
    {
        return false;
    }

    let (order_price, market_price) = (order.trigger_price.clone().unwrap().rate, market_price);

    match (&order.order_type, &order.position) {
        (PerpetualOrderType::LimitOpen, PerpetualPosition::Long) => market_price <= order_price,
        (PerpetualOrderType::LimitOpen, PerpetualPosition::Short) => market_price >= order_price,
        (PerpetualOrderType::LimitClose, PerpetualPosition::Long) => market_price >= order_price,
        (PerpetualOrderType::LimitClose, PerpetualPosition::Short) => market_price <= order_price,
        (PerpetualOrderType::StopLoss, PerpetualPosition::Long) => market_price <= order_price,
        (PerpetualOrderType::StopLoss, PerpetualPosition::Short) => market_price >= order_price,
        _ => false,
    }
}

fn check_spot_order(order: &SpotOrder, market_price: Decimal) -> bool {
    if order.order_type == SpotOrderType::MarketBuy {
        return false;
    }

    let order_price = order.order_price.rate;

    let market_price = if order.order_type == SpotOrderType::LimitBuy {
        match Decimal::one().checked_div(market_price.clone()) {
            Ok(market_price) => market_price,
            Err(_) => return false,
        }
    } else {
        market_price
    };

    match order.order_type {
        SpotOrderType::LimitBuy => market_price <= order_price,
        SpotOrderType::LimitSell => market_price >= order_price,
        SpotOrderType::StopLoss => market_price <= order_price,
        _ => false,
    }
}

fn process_spot_order(
    order: &SpotOrder,
    submsgs: &mut Vec<SubMsg<ElysMsg>>,
    sender: &str,
    reply_info_id: &mut u64,
    amm_swap_estimation: AmmSwapEstimationByDenomResponse,
    storage: &mut dyn Storage,
    discount: Decimal,
) -> StdResult<()> {
    let token_out_min_amount: Int128 = match order.order_type {
        SpotOrderType::LimitBuy => calculate_token_out_min_amount(order),
        SpotOrderType::LimitSell => calculate_token_out_min_amount(order),
        SpotOrderType::StopLoss => Int128::zero(),
        _ => Int128::zero(),
    };

    let msg = ElysMsg::amm_swap_exact_amount_in(
        sender,
        &order.order_amount,
        &amm_swap_estimation.in_route.unwrap(),
        token_out_min_amount,
        discount,
        &order.owner_address,
    );

    *reply_info_id = match reply_info_id.checked_add(1) {
        Some(id) => id,
        None => {
            return Err(StdError::overflow(OverflowError::new(
                cosmwasm_std::OverflowOperation::Add,
                "reply_info_max_id",
                "increment one",
            ))
            .into())
        }
    };

    let reply_info = ReplyInfo {
        id: *reply_info_id,
        reply_type: ReplyType::SpotOrder,
        data: Some(to_json_binary(&order.order_id)?),
    };

    submsgs.push(SubMsg::reply_always(msg, *reply_info_id));

    REPLY_INFO.save(storage, *reply_info_id, &reply_info)?;

    Ok(())
}

fn calculate_token_out_min_amount(_order: &SpotOrder) -> Int128 {
    // FIXME:
    // insteade we want to use the amount field from swap-estimation-by-denom that
    // include slippage and reduce it by 1% that should be our token out min amount to return here
    //
    // let SpotOrder {
    //     order_amount,
    //     order_price,
    //     ..
    // } = order;

    // let amount = if order_amount.denom == order_price.base_denom {
    //     order_amount.amount * order_price.rate
    // } else {
    //     order_amount.amount * Decimal::one().div(order_price.rate)
    // };

    // Int128::new((amount.u128()) as i128)
    Int128::zero()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{coin, Addr, Timestamp};

    use super::*;

    #[test]
    fn test_check_spot_order_limit_buy() {
        // Arrange
        let spot_order = SpotOrder {
            order_type: SpotOrderType::LimitBuy,
            order_id: 1,
            order_price: OrderPrice {
                base_denom: "uatom".to_string(),
                quote_denom: "usdc".to_string(),
                rate: Decimal::from_str("0.1").unwrap(),
            },
            order_amount: coin(1000000, "usdc"),

            owner_address: Addr::unchecked("elysd"),
            order_target_denom: "uatom".to_string(),
            status: Status::Pending,
            date: Date {
                height: 5,
                time: Timestamp::from_seconds(5),
            },
            // Initialize the rest of the SpotOrder fields here
        };
        let market_price = Decimal::from_str("9.29").unwrap();

        // Act
        let result = check_spot_order(&spot_order, market_price);

        // Assert
        assert_eq!(result, false); // Change as needed
    }

    #[test]
    fn test_check_spot_order_limit_sell() {
        // Arrange
        let spot_order = SpotOrder {
            order_type: SpotOrderType::LimitSell,
            order_id: 1,
            order_price: OrderPrice {
                base_denom: "uatom".to_string(),
                quote_denom: "usdc".to_string(),
                rate: Decimal::from_str("0.111111").unwrap(),
            },
            order_amount: coin(1000000, "uatom"),

            owner_address: Addr::unchecked("elysd"),
            order_target_denom: "usdc".to_string(),
            status: Status::Pending,
            date: Date {
                height: 5,
                time: Timestamp::from_seconds(5),
            },
            // Initialize the rest of the SpotOrder fields here
        };
        let market_price = Decimal::from_str("9.29").unwrap();
        // Act
        let result = check_spot_order(&spot_order, market_price);

        // Assert
        assert_eq!(result, true); // Change as needed
    }
}
