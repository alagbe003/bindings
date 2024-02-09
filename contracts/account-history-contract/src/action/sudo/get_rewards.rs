use cosmwasm_std::{coin, Decimal, Deps, StdError, StdResult, Uint128};
use elys_bindings::{ElysQuerier, ElysQuery};

use crate::{
    msg::query_resp::GetRewardsResp,
    types::{AccountSnapshot, ElysDenom, Reward},
};

pub fn get_rewards(deps: Deps<ElysQuery>, address: String) -> StdResult<GetRewardsResp> {
    let querier = ElysQuerier::new(&deps.querier);
    let commitments = querier.get_commitments(address)?;

    let denom_usdc_entry = querier.get_asset_profile(ElysDenom::Usdc.as_str().to_string())?;
    let denom_uusdc = denom_usdc_entry.entry.denom;
    let usdc_display_denom = denom_usdc_entry.entry.display_name;

    let denom_uelys = ElysDenom::Elys.as_str().to_string();
    let denom_ueden = ElysDenom::Eden.as_str().to_string();
    let denom_uedenb = ElysDenom::EdenBoost.as_str().to_string();

    let usdc_oracle_price = querier.get_oracle_price(
        usdc_display_denom.clone(),
        ElysDenom::AnySource.as_str().to_string(),
        0,
    )?;
    let usdc_price = usdc_oracle_price
        .price
        .price
        .checked_div(Decimal::from_atomics(Uint128::new(1000000), 0).unwrap())
        .unwrap();

    let mut rewards = Reward {
        usdc_usd: Decimal::zero(),
        eden_usd: Decimal::zero(),
        eden_boost: Uint128::zero(),
        other_usd: Decimal::zero(),
        total_usd: Decimal::zero(),
    };

    match commitments.commitments.rewards_unclaimed {
        Some(rewards_unclaimed) => {
            for reward in rewards_unclaimed {
                // uusdc
                if reward.denom == denom_uusdc {
                    let usdc_rewards = Decimal::from_atomics(reward.amount, 0).unwrap();
                    rewards.usdc_usd = usdc_rewards.checked_mul(usdc_price).unwrap();
                    rewards.total_usd = rewards.total_usd.checked_add(rewards.usdc_usd).unwrap();

                    continue;
                }

                // ueden
                if reward.denom == denom_ueden {
                    // if it is eden, we should elys denom instead of ueden as it is not available in LP pool and has the same value with elys.
                    let reward_in_elys = coin(reward.amount.u128(), denom_uelys.to_owned());
                    let price = querier.get_amm_price_by_denom(
                        coin(1000000, reward_in_elys.denom),
                        Decimal::zero(),
                    )?;

                    let amount = coin(
                        (price
                            .checked_mul(Decimal::from_atomics(reward_in_elys.amount, 0).map_err(
                                |_| StdError::generic_err(format!("failed to convert to decimal")),
                            )?)
                            .map_err(|e| {
                                StdError::generic_err(format!(
                                    "failed to get_amm_price_by_denom: {}",
                                    e
                                ))
                            })?)
                        .to_uint_floor()
                        .u128(),
                        &denom_uusdc,
                    );
                    let rewards_in_usdc = Decimal::from_atomics(amount.amount, 0).unwrap();
                    rewards.eden_usd = rewards_in_usdc.checked_mul(usdc_price).unwrap();
                    rewards.total_usd = rewards.total_usd.checked_add(rewards.eden_usd).unwrap();
                    continue;
                }

                // uedenb - we don't value eden boost in usd.
                if reward.denom == denom_uedenb {
                    rewards.eden_boost = reward.amount;
                    continue;
                }

                // We accumulate other denoms in a single usd.
                let price = querier
                    .get_amm_price_by_denom(coin(1000000, &reward.denom), Decimal::zero())?;

                let amount = coin(
                    (price
                        .checked_mul(Decimal::from_atomics(reward.amount, 0).map_err(|_| {
                            StdError::generic_err(format!("failed to convert to decimal"))
                        })?)
                        .map_err(|e| {
                            StdError::generic_err(format!(
                                "failed to get_amm_price_by_denom: {}",
                                e
                            ))
                        })?)
                    .to_uint_floor()
                    .u128(),
                    &denom_uusdc,
                );
                let rewards_in_usdc = Decimal::from_atomics(amount.amount, 0).unwrap();
                let rewards_in_usd = rewards_in_usdc.checked_mul(usdc_price).unwrap();

                rewards.other_usd = rewards.other_usd.checked_add(rewards_in_usd).unwrap();
                rewards.total_usd = rewards.total_usd.checked_add(rewards_in_usd).unwrap();
            }
        }
        None => {
            return Ok(GetRewardsResp {
                rewards: AccountSnapshot::zero(&denom_uusdc).reward,
            });
        }
    }

    let resp = GetRewardsResp { rewards: rewards };
    Ok(resp)
}