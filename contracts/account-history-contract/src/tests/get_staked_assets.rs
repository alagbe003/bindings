use std::str::FromStr;

use crate::states::{EXPIRATION, PAGINATION, TRADE_SHIELD_ADDRESS, VALUE_DENOM};
use crate::tests::get_staked_assets::query_resp::StakedAssetsResponse;
use crate::types::earn_program::{
    EdenBoostEarnProgram, EdenEarnProgram, ElysEarnProgram, UsdcEarnProgram,
};
use crate::types::{AprElys, AprUsdc, BalanceReward, StakedAssets};
use crate::{
    entry_point::{execute, query, sudo},
    msg::*,
};
use anyhow::{bail, Error, Result as AnyResult};
use cosmwasm_std::{
    coins, to_json_binary, Addr, DecCoin, Decimal, Decimal256, DepsMut, Empty, Env, Int128,
    MessageInfo, Response, StdError, StdResult, Timestamp, Uint128,
};
use cw_multi_test::{AppResponse, BasicAppBuilder, ContractWrapper, Executor, Module};
use elys_bindings::query_resp::{
    BalanceBorrowed, Entry, Lockup, QueryAprResponse, QueryGetEntryResponse, QueryGetPriceResponse,
    QueryStakedPositionResponse, QueryUnstakedPositionResponse, QueryVestingInfoResponse,
    StakedAvailable,
};
use elys_bindings::types::{
    BalanceAvailable, PageRequest, Price, StakedPosition, StakingValidator, UnstakedPosition,
};
use elys_bindings::{ElysMsg, ElysQuery};
use elys_bindings_test::{
    ElysModule, ACCOUNT, ASSET_INFO, LAST_MODULE_USED, PERPETUAL_OPENED_POSITION, PRICES,
};
use trade_shield_contract::entry_point::{
    execute as trade_shield_execute, instantiate as trade_shield_init, query as trade_shield_query,
};
use trade_shield_contract::msg::InstantiateMsg as TradeShieldInstantiateMsg;

fn mock_instantiate(
    deps: DepsMut<ElysQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response<ElysMsg>> {
    EXPIRATION.save(deps.storage, &msg.expiration)?;
    PAGINATION.save(
        deps.storage,
        &PageRequest {
            key: None,
            limit: msg.limit,
            reverse: false,
            offset: None,
            count_total: false,
        },
    )?;
    VALUE_DENOM.save(deps.storage, &msg.value_denom)?;
    TRADE_SHIELD_ADDRESS.save(deps.storage, &msg.trade_shield_address)?;
    Ok(Response::new())
}

struct ElysModuleWrapper(ElysModule);

impl Module for ElysModuleWrapper {
    type QueryT = ElysQuery;
    type ExecT = ElysMsg;
    type SudoT = Empty;

    fn query(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &dyn cosmwasm_std::Storage,
        querier: &dyn cosmwasm_std::Querier,
        block: &cosmwasm_std::BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<cosmwasm_std::Binary> {
        match request {
            ElysQuery::AssetProfileEntry { base_denom } => {
                let resp = match base_denom.as_str() {
                    "uusdc" => QueryGetEntryResponse {
                        entry: Entry {
                            address: "".to_string(),
                            authority: "elys10d07y265gmmuvt4z0w9aw880jnsr700j6z2zm3".to_string(),
                            base_denom: "uusdc".to_string(),
                            commit_enabled: true,
                            decimals: 6,
                            denom: "ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65".to_string(),
                            display_name: "USDC".to_string(),
                            display_symbol: "uUSDC".to_string(),
                            external_symbol: "uUSDC".to_string(),
                            ibc_channel_id: "channel-12".to_string(),
                            ibc_counterparty_chain_id: "".to_string(),
                            ibc_counterparty_channel_id: "channel-19".to_string(),
                            ibc_counterparty_denom: "".to_string(),
                            network: "".to_string(),
                            path: "transfer/channel-12".to_string(),
                            permissions: vec![],
                            transfer_limit: "".to_string(),
                            unit_denom: "uusdc".to_string(),
                            withdraw_enabled: true,
                        },
                    },
                    "ueden" => QueryGetEntryResponse {
                        entry: Entry {
                            address: "".to_string(),
                            authority: "elys10d07y265gmmuvt4z0w9aw880jnsr700j6z2zm3".to_string(),
                            base_denom: "ueden".to_string(),
                            commit_enabled: true,
                            decimals: 6,
                            denom: "ueden".to_string(),
                            display_name: "EDEN".to_string(),
                            display_symbol: "".to_string(),
                            external_symbol: "".to_string(),
                            ibc_channel_id: "".to_string(),
                            ibc_counterparty_chain_id: "".to_string(),
                            ibc_counterparty_channel_id: "".to_string(),
                            ibc_counterparty_denom: "".to_string(),
                            network: "".to_string(),
                            path: "".to_string(),
                            permissions: vec![],
                            transfer_limit: "".to_string(),
                            unit_denom: "".to_string(),
                            withdraw_enabled: true,
                        },
                    },
                    "uelys" => QueryGetEntryResponse {
                        entry: Entry {
                            address: "".to_string(),
                            authority: "elys10d07y265gmmuvt4z0w9aw880jnsr700j6z2zm3".to_string(),
                            base_denom: "uelys".to_string(),
                            commit_enabled: true,
                            decimals: 6,
                            denom: "uelys".to_string(),
                            display_name: "ELYS".to_string(),
                            display_symbol: "".to_string(),
                            external_symbol: "".to_string(),
                            ibc_channel_id: "".to_string(),
                            ibc_counterparty_chain_id: "".to_string(),
                            ibc_counterparty_channel_id: "".to_string(),
                            ibc_counterparty_denom: "".to_string(),
                            network: "".to_string(),
                            path: "".to_string(),
                            permissions: vec![],
                            transfer_limit: "".to_string(),
                            unit_denom: "".to_string(),
                            withdraw_enabled: true,
                        },
                    },
                    _ => return Err(Error::new(StdError::not_found(base_denom))),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::AmmPriceByDenom { token_in, .. } => {
                let spot_price = match token_in.denom.as_str() {
                    "uelys" => Decimal::from_str("0.297883685357378504").unwrap(),
                    _ => return Err(Error::new(StdError::not_found(token_in.denom.as_str()))),
                };
                Ok(to_json_binary(&spot_price)?)
            }
            ElysQuery::OraclePrice { asset, .. } => {
                let resp = match asset.as_str() {
                    "USDC" => QueryGetPriceResponse {
                        price: Price {
                            asset: "USDC".to_string(),
                            price: Decimal::one(),
                            source: "uelys".to_string(),
                            provider: "elys1wzm8dvpxpxxf26y4xn85w5adakcenprg4cq2uf".to_string(),
                            // set timestamp to now
                            timestamp: block.time.seconds(),
                        },
                    },
                    _ => return Err(Error::new(StdError::not_found(asset))),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::CommitmentRewardsSubBucketBalanceOfDenom { denom, program, .. } => {
                let resp: BalanceAvailable = match (denom.as_str(), program) {
                    ("ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65", 1) => {
                        BalanceAvailable {
                            amount: Uint128::zero(),
                            usd_amount: Decimal::zero(),
                        }
                    }
                    ("ueden", 1) => BalanceAvailable {
                        amount: Uint128::new(349209420),
                        usd_amount: Decimal::from_str("349209420").unwrap(),
                    },
                    ("ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65", 2) => {
                        BalanceAvailable {
                            amount: Uint128::zero(),
                            usd_amount: Decimal::zero(),
                        }
                    }
                    ("ueden", 2) => BalanceAvailable {
                        amount: Uint128::new(9868),
                        usd_amount: Decimal::from_str("9868").unwrap(),
                    },
                    ("uedenb", 2) => BalanceAvailable {
                        amount: Uint128::new(654083056),
                        usd_amount: Decimal::from_str("654083056").unwrap(),
                    },
                    ("ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65", 3) => {
                        BalanceAvailable {
                            amount: Uint128::new(1161),
                            usd_amount: Decimal::from_str("1161").unwrap(),
                        }
                    }
                    ("ueden", 3) => BalanceAvailable {
                        amount: Uint128::new(2984882),
                        usd_amount: Decimal::from_str("2984882").unwrap(),
                    },
                    ("uedenb", 3) => BalanceAvailable {
                        amount: Uint128::new(10155052),
                        usd_amount: Decimal::from_str("10155052").unwrap(),
                    },
                    ("ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65", 4) => {
                        BalanceAvailable {
                            amount: Uint128::zero(),
                            usd_amount: Decimal::zero(),
                        }
                    }
                    ("ueden", 4) => BalanceAvailable {
                        amount: Uint128::zero(),
                        usd_amount: Decimal::zero(),
                    },
                    ("uedenb", 4) => BalanceAvailable {
                        amount: Uint128::zero(),
                        usd_amount: Decimal::zero(),
                    },
                    _ => return Err(Error::new(StdError::not_found(denom))),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::CommitmentStakedPositions { delegator_address } => {
                let resp = match delegator_address.as_str() {
                    "user" => QueryStakedPositionResponse {
                        staked_position: Some(vec![StakedPosition {
                            id: "2".to_string(),
                            validator: StakingValidator {
                                address: "elysvaloper1ng8sen6z5xzcfjtyrsedpe43hglymq040x3cpw"
                                    .to_string(),
                                name: "nirvana".to_string(),
                                voting_power: Decimal::from_str("25.6521469796402094").unwrap(),
                                commission: Decimal::from_str("0.1").unwrap(),
                                profile_picture_src: Some("https://elys.network".to_string()),
                            },
                            staked: BalanceAvailable {
                                amount: Uint128::new(10000000),
                                usd_amount: Decimal::from_str("10000000").unwrap(),
                            },
                        }]),
                    },
                    _ => return Err(Error::new(StdError::not_found(delegator_address))),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::CommitmentUnStakedPositions { delegator_address } => {
                let resp = match delegator_address.as_str() {
                    "user" => QueryUnstakedPositionResponse {
                        unstaked_position: Some(vec![UnstakedPosition {
                            id: "1".to_string(),
                            validator: StakingValidator {
                                address: "elysvaloper1ng8sen6z5xzcfjtyrsedpe43hglymq040x3cpw"
                                    .to_string(),
                                name: "nirvana".to_string(),
                                voting_power: Decimal::from_str("25.6521469796402094").unwrap(),
                                commission: Decimal::from_str("0.1").unwrap(),
                                profile_picture_src: Some("https://elys.network".to_string()),
                            },
                            remaining_time: 1707328694,
                            unstaked: BalanceAvailable {
                                amount: Uint128::new(100038144098),
                                usd_amount: Decimal::from_str("100038144098").unwrap(),
                            },
                        }]),
                    },
                    _ => return Err(Error::new(StdError::not_found(delegator_address))),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::AmmBalance { address, denom } => {
                let resp = match (address.as_str(), denom.as_str()) {
                    (
                        "user",
                        "ibc/0E1517E2771CA7C03F2ED3F9BAECCAEADF0BFD79B89679E834933BC0F179AD98",
                    ) => BalanceAvailable {
                        amount: Uint128::new(21798000),
                        usd_amount: Decimal::from_str("21798000").unwrap(),
                    },
                    (
                        "user",
                        "ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65",
                    ) => BalanceAvailable {
                        amount: Uint128::new(5333229342748),
                        usd_amount: Decimal::from_str("5333229342748").unwrap(),
                    },
                    (
                        "user",
                        "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                    ) => BalanceAvailable {
                        amount: Uint128::new(2704998),
                        usd_amount: Decimal::from_str("2704998").unwrap(),
                    },
                    (
                        "user",
                        "ibc/2FBCFC209420E6CECED6EE0BC599E74349759352CE953E27A6871BB3D84BC058",
                    ) => BalanceAvailable {
                        amount: Uint128::new(594000000000200000),
                        usd_amount: Decimal::from_str("594000000000200000").unwrap(),
                    },
                    (
                        "user",
                        "ibc/326A89923D85047E6418A671FBACCAFA2686B01A16ED4A0AD92954FCE1485910",
                    ) => BalanceAvailable {
                        amount: Uint128::new(1085352),
                        usd_amount: Decimal::from_str("1085352").unwrap(),
                    },
                    (
                        "user",
                        "ibc/43881AB3B3D05FD9D3606D7F57CBE6EEEA89D18AC66AF9E2915ED43940E71CFD",
                    ) => BalanceAvailable {
                        amount: Uint128::new(168400000000000000),
                        usd_amount: Decimal::from_str("168400000000000000").unwrap(),
                    },
                    (
                        "user",
                        "ibc/4DAE26570FD24ABA40E2BE4137E39D946C78B00B248D3F78B0919567C4371156",
                    ) => BalanceAvailable {
                        amount: Uint128::new(49765000),
                        usd_amount: Decimal::from_str("49765000").unwrap(),
                    },
                    (
                        "user",
                        "ibc/977D5388D2FBE72D9A33FE2423BF8F4DADF3B591207CC98A295B9ACF81E4DE40",
                    ) => BalanceAvailable {
                        amount: Uint128::new(9100000),
                        usd_amount: Decimal::from_str("9100000").unwrap(),
                    },
                    (
                        "user",
                        "ibc/E059CD828E5009D4CF03C4494BEA73749250287FC98DD46E19F9016B918BF49D",
                    ) => BalanceAvailable {
                        amount: Uint128::new(141000000000000000),
                        usd_amount: Decimal::from_str("141000000000000000").unwrap(),
                    },
                    (
                        "user",
                        "ibc/E2D2F6ADCC68AA3384B2F5DFACCA437923D137C14E86FB8A10207CF3BED0C8D4",
                    ) => BalanceAvailable {
                        amount: Uint128::new(37403942),
                        usd_amount: Decimal::from_str("37403942").unwrap(),
                    },
                    (
                        "user",
                        "ibc/FB22E35236996F6B0B1C9D407E8A379A7B1F4083F1960907A1622F022AE450E1",
                    ) => BalanceAvailable {
                        amount: Uint128::new(79979999999749000),
                        usd_amount: Decimal::from_str("79979999999749000").unwrap(),
                    },
                    ("user", "uelys") => BalanceAvailable {
                        amount: Uint128::new(45666543),
                        usd_amount: Decimal::from_str("45666543").unwrap(),
                    },
                    _ => BalanceAvailable {
                        amount: Uint128::zero(),
                        usd_amount: Decimal::zero(),
                    },
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::CommitmentStakedBalanceOfDenom { denom, .. } => {
                let resp: StakedAvailable = match denom.as_str() {
                    "ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65" => {
                        StakedAvailable {
                            usd_amount: Decimal::zero(),
                            amount: Uint128::zero(),
                            lockups: None,
                        }
                    }
                    "uelys" => StakedAvailable {
                        usd_amount: Decimal::from_str("10000000").unwrap(),
                        amount: Uint128::new(10000000),
                        lockups: Some(vec![]),
                    },
                    "ueden" => StakedAvailable {
                        usd_amount: Decimal::from_str("2587611057").unwrap(),
                        amount: Uint128::new(2587611057),
                        lockups: Some(vec![Lockup {
                            amount: Int128::new(5200770174),
                            // use now time
                            unlock_timestamp: block.time.seconds(),
                        }]),
                    },
                    "uedenb" => StakedAvailable {
                        usd_amount: Decimal::zero(),
                        amount: Uint128::zero(),
                        lockups: None,
                    },
                    _ => return Err(Error::new(StdError::not_found(denom))),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::StableStakeBalanceOfBorrow {} => {
                let resp = BalanceBorrowed {
                    usd_amount: Decimal::from_str("204000000001").unwrap(),
                    percentage: Decimal::one(),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::IncentiveApr {
                withdraw_type,
                denom,
            } => {
                let resp: QueryAprResponse = match (withdraw_type, denom.as_str()) {
                    (1, "uusdc") => QueryAprResponse {
                        apr: Uint128::new(100),
                    },
                    (1, "ueden") => QueryAprResponse {
                        apr: Uint128::new(168),
                    },
                    (4, "uusdc") => QueryAprResponse {
                        apr: Uint128::zero(),
                    },
                    (4, "ueden") => QueryAprResponse {
                        apr: Uint128::new(29),
                    },
                    (3, "uusdc") => QueryAprResponse {
                        apr: Uint128::zero(),
                    },
                    (3, "ueden") => QueryAprResponse {
                        apr: Uint128::new(29),
                    },
                    (3, "uedenb") => QueryAprResponse {
                        apr: Uint128::new(100),
                    },
                    (2, "uusdc") => QueryAprResponse {
                        apr: Uint128::zero(),
                    },
                    (2, "ueden") => QueryAprResponse {
                        apr: Uint128::new(29),
                    },
                    (2, "uedenb") => QueryAprResponse {
                        apr: Uint128::new(100),
                    },
                    _ => return Err(Error::new(StdError::not_found(denom))),
                };
                Ok(to_json_binary(&resp)?)
            }
            ElysQuery::CommitmentVestingInfo { .. } => {
                let resp = QueryVestingInfoResponse {
                    vesting: BalanceAvailable {
                        amount: Uint128::zero(),
                        usd_amount: Decimal::zero(),
                    },
                    vesting_details: Some(vec![]),
                };
                Ok(to_json_binary(&resp)?)
            }
            _ => self.0.query(api, storage, querier, block, request),
        }
    }

    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn cosmwasm_std::Storage,
        router: &dyn cw_multi_test::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        match msg {
            _ => self.0.execute(api, storage, router, block, sender, msg),
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &mut dyn cosmwasm_std::Storage,
        _router: &dyn cw_multi_test::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &cosmwasm_std::BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        bail!("sudo is not implemented for ElysModule")
    }
}

#[test]
fn get_staked_assets() {
    // Create a wallet for the "user" with an initial balance of 100 usdc
    let wallet = vec![(
        "user",
        coins(
            200__000_000,
            "ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65",
        ),
    )];

    let mut addresses: Vec<String> = vec![];
    let mut app = BasicAppBuilder::<ElysMsg, ElysQuery>::new_custom()
        .with_custom(ElysModuleWrapper(ElysModule {}))
        .build(|roouter, _, storage| {
            for (wallet_owner, wallet_contenent) in wallet {
                roouter
                    .bank
                    .init_balance(storage, &Addr::unchecked(wallet_owner), wallet_contenent)
                    .unwrap();
                addresses.push(wallet_owner.to_owned())
            }
            ACCOUNT.save(storage, &addresses).unwrap();
            PERPETUAL_OPENED_POSITION.save(storage, &vec![]).unwrap();
            ASSET_INFO.save(storage, &vec![]).unwrap();
            PRICES.save(storage, &vec![]).unwrap();
            LAST_MODULE_USED.save(storage, &None).unwrap();
        });

    // trade shield deployment
    let trade_shield_code =
        ContractWrapper::new(trade_shield_execute, trade_shield_init, trade_shield_query);
    let trade_shield_code_id = app.store_code(Box::new(trade_shield_code));
    let trade_shield_init = TradeShieldInstantiateMsg {};
    let trade_shield_address = app
        .instantiate_contract(
            trade_shield_code_id,
            Addr::unchecked("owner"),
            &trade_shield_init,
            &[],
            "Contract",
            None,
        )
        .unwrap()
        .to_string();

    // Create a contract wrapper and store its code.
    let code = ContractWrapper::new(execute, mock_instantiate, query).with_sudo(sudo);
    let code_id = app.store_code(Box::new(code));

    // Create a mock message to instantiate the contract with no initial orders.
    let instantiate_msg = InstantiateMsg {
        limit: 3,
        expiration: cw_utils::Expiration::AtTime(Timestamp::from_seconds(604800)),
        value_denom: "ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65"
            .to_string(),
        trade_shield_address,
    };

    // Instantiate the contract with "owner" as the deployer.
    let addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &instantiate_msg,
            &[],
            "Contract",
            None,
        )
        .unwrap();

    app.wasm_sudo(addr.clone(), &SudoMsg::ClockEndBlock {})
        .unwrap();

    // Query the contract for the existing order.
    let resp: StakedAssetsResponse = app
        .wrap()
        .query_wasm_smart(
            &addr,
            &QueryMsg::GetStakedAssets {
                user_address: "user".to_string(),
            },
        )
        .unwrap();

    let expected: StakedAssetsResponse = StakedAssetsResponse {
        total_staked_balance: DecCoin::new(
            Decimal256::from_str("773.785954784235398524").unwrap(),
            "ibc/2180E84E20F5679FCC760D8C165B60F42065DEF7F46A72B447CFF1B7DC6C0A65".to_string(),
        ),
        staked_assets: StakedAssets {
            eden_boost_earn_program: EdenBoostEarnProgram {
                bonding_period: 0,
                apr: AprUsdc {
                    uusdc: Uint128::zero(),
                    ueden: Uint128::new(29),
                },
                available: Some(Uint128::zero()),
                staked: Some(Uint128::zero()),
                rewards: Some(vec![
                    BalanceReward {
                        asset: "uusdc".to_string(),
                        amount: Uint128::zero(),
                        usd_amount: Some(Decimal::zero()),
                    },
                    BalanceReward {
                        asset: "ueden".to_string(),
                        amount: Uint128::zero(),
                        usd_amount: Some(Decimal::zero()),
                    },
                ]),
            },
            eden_earn_program: EdenEarnProgram {
                bonding_period: 0,
                apr: AprElys {
                    uusdc: Uint128::zero(),
                    ueden: Uint128::new(29),
                    uedenb: Uint128::new(100),
                },
                available: Some(BalanceAvailable {
                    amount: Uint128::zero(),
                    usd_amount: Decimal::zero(),
                }),
                staked: Some(StakedAvailable {
                    usd_amount: Decimal::from_str("770.807117930661613484").unwrap(),
                    amount: Uint128::new(2587611057),
                    lockups: Some(vec![Lockup {
                        amount: Int128::new(5200770174),
                        unlock_timestamp: 1571797419,
                    }]),
                }),
                rewards: Some(vec![
                    BalanceReward {
                        asset: "uusdc".to_string(),
                        amount: Uint128::new(1161),
                        usd_amount: Some(Decimal::from_str("0.001161").unwrap()),
                    },
                    BalanceReward {
                        asset: "ueden".to_string(),
                        amount: Uint128::new(2984882),
                        usd_amount: Some(Decimal::from_str("0.889147650516902663").unwrap()),
                    },
                    BalanceReward {
                        asset: "uedenb".to_string(),
                        amount: Uint128::new(10155052),
                        usd_amount: None,
                    },
                ]),
                vesting: Some(BalanceAvailable {
                    amount: Uint128::zero(),
                    usd_amount: Decimal::zero(),
                }),
                vesting_details: Some(vec![]), // FIXME: according to Wari we should have vesting details here
            },
            elys_earn_program: ElysEarnProgram {
                bonding_period: 14,
                apr: AprElys {
                    uusdc: Uint128::zero(),
                    ueden: Uint128::new(29),
                    uedenb: Uint128::new(100),
                },
                available: Some(BalanceAvailable {
                    amount: Uint128::new(45666543),
                    usd_amount: Decimal::from_str("13.60331812637119582").unwrap(),
                }),
                staked: Some(StakedAvailable {
                    usd_amount: Decimal::from_str("2.97883685357378504").unwrap(),
                    amount: Uint128::new(10000000),
                    lockups: Some(vec![]),
                }),
                rewards: Some(vec![
                    BalanceReward {
                        asset: "uusdc".to_string(),
                        amount: Uint128::zero(),
                        usd_amount: Some(Decimal::zero()),
                    },
                    BalanceReward {
                        asset: "ueden".to_string(),
                        amount: Uint128::new(9868),
                        usd_amount: Some(Decimal::from_str("0.002939516207106611").unwrap()),
                    },
                    BalanceReward {
                        asset: "uedenb".to_string(),
                        amount: Uint128::new(654083056),
                        usd_amount: None,
                    },
                ]),
                staked_positions: Some(vec![StakedPosition {
                    id: "2".to_string(),
                    validator: StakingValidator {
                        address: "elysvaloper1ng8sen6z5xzcfjtyrsedpe43hglymq040x3cpw".to_string(),
                        name: "nirvana".to_string(),
                        voting_power: Decimal::from_str("25.6521469796402094").unwrap(),
                        commission: Decimal::from_str("0.1").unwrap(),
                        profile_picture_src: Some("https://elys.network".to_string()),
                    },
                    staked: BalanceAvailable {
                        amount: Uint128::new(10000000),
                        usd_amount: Decimal::from_str("2.97883685357378504").unwrap(),
                    },
                }]),
                unstaked_positions: Some(vec![UnstakedPosition {
                    id: "1".to_string(),
                    validator: StakingValidator {
                        address: "elysvaloper1ng8sen6z5xzcfjtyrsedpe43hglymq040x3cpw".to_string(),
                        name: "nirvana".to_string(),
                        voting_power: Decimal::from_str("25.6521469796402094").unwrap(),
                        commission: Decimal::from_str("0.1").unwrap(),
                        profile_picture_src: Some("https://elys.network".to_string()),
                    },
                    remaining_time: 1707328694000,
                    unstaked: BalanceAvailable {
                        amount: Uint128::new(100038144098),
                        usd_amount: Decimal::from_str("29799.731040224723410679").unwrap(),
                    },
                }]),
            },
            usdc_earn_program: UsdcEarnProgram {
                bonding_period: 0,
                apr: AprUsdc {
                    uusdc: Uint128::new(100),
                    ueden: Uint128::new(168),
                },
                available: Some(BalanceAvailable {
                    amount: Uint128::new(5333229342748),
                    usd_amount: Decimal::from_str("5333229.342748").unwrap(),
                }),
                staked: Some(StakedAvailable {
                    usd_amount: Decimal::zero(),
                    amount: Uint128::zero(),
                    lockups: None,
                }),
                rewards: Some(vec![
                    BalanceReward {
                        asset: "uusdc".to_string(),
                        amount: Uint128::zero(),
                        usd_amount: Some(Decimal::zero()),
                    },
                    BalanceReward {
                        asset: "ueden".to_string(),
                        amount: Uint128::new(349209420),
                        usd_amount: Some(Decimal::from_str("104.023788991112640102").unwrap()),
                    },
                ]),
                borrowed: Some(BalanceBorrowed {
                    usd_amount: Decimal::from_str("204000.000001").unwrap(),
                    percentage: Decimal::one(),
                }),
            },
        },
    };

    // test if the response is the same as the expected

    // staked assets

    // USDC program
    assert_eq!(
        resp.staked_assets.usdc_earn_program.bonding_period,
        expected.staked_assets.usdc_earn_program.bonding_period
    );
    assert_eq!(
        resp.staked_assets.usdc_earn_program.apr,
        expected.staked_assets.usdc_earn_program.apr
    );
    assert_eq!(
        resp.staked_assets.usdc_earn_program.available,
        expected.staked_assets.usdc_earn_program.available
    );
    assert_eq!(
        resp.staked_assets.usdc_earn_program.staked,
        expected.staked_assets.usdc_earn_program.staked
    );
    assert_eq!(
        resp.staked_assets.usdc_earn_program.rewards,
        expected.staked_assets.usdc_earn_program.rewards
    );
    assert_eq!(
        resp.staked_assets.usdc_earn_program.borrowed,
        expected.staked_assets.usdc_earn_program.borrowed
    );
    assert_eq!(
        resp.staked_assets.usdc_earn_program,
        expected.staked_assets.usdc_earn_program
    );

    // ELYS program
    assert_eq!(
        resp.staked_assets.elys_earn_program.bonding_period,
        expected.staked_assets.elys_earn_program.bonding_period
    );
    assert_eq!(
        resp.staked_assets.elys_earn_program.apr,
        expected.staked_assets.elys_earn_program.apr
    );
    assert_eq!(
        resp.staked_assets.elys_earn_program.available,
        expected.staked_assets.elys_earn_program.available
    );
    assert_eq!(
        resp.staked_assets.elys_earn_program.staked,
        expected.staked_assets.elys_earn_program.staked
    );
    assert_eq!(
        resp.staked_assets.elys_earn_program.rewards,
        expected.staked_assets.elys_earn_program.rewards
    );
    assert_eq!(
        resp.staked_assets.elys_earn_program.staked_positions,
        expected.staked_assets.elys_earn_program.staked_positions
    );
    assert_eq!(
        resp.staked_assets.elys_earn_program.unstaked_positions,
        expected.staked_assets.elys_earn_program.unstaked_positions
    );
    assert_eq!(
        resp.staked_assets.elys_earn_program,
        expected.staked_assets.elys_earn_program
    );

    // EDEN program
    assert_eq!(
        resp.staked_assets.eden_earn_program.bonding_period,
        expected.staked_assets.eden_earn_program.bonding_period
    );
    assert_eq!(
        resp.staked_assets.eden_earn_program.apr,
        expected.staked_assets.eden_earn_program.apr
    );
    assert_eq!(
        resp.staked_assets.eden_earn_program.available,
        expected.staked_assets.eden_earn_program.available
    );
    assert_eq!(
        resp.staked_assets.eden_earn_program.staked,
        expected.staked_assets.eden_earn_program.staked
    );
    assert_eq!(
        resp.staked_assets.eden_earn_program.rewards,
        expected.staked_assets.eden_earn_program.rewards
    );
    assert_eq!(
        resp.staked_assets.eden_earn_program.vesting,
        expected.staked_assets.eden_earn_program.vesting
    );
    assert_eq!(
        resp.staked_assets.eden_earn_program.vesting_details,
        expected.staked_assets.eden_earn_program.vesting_details
    );
    assert_eq!(
        resp.staked_assets.eden_earn_program,
        expected.staked_assets.eden_earn_program
    );

    // EDEN BOOST program
    assert_eq!(
        resp.staked_assets.eden_boost_earn_program.bonding_period,
        expected
            .staked_assets
            .eden_boost_earn_program
            .bonding_period
    );
    assert_eq!(
        resp.staked_assets.eden_boost_earn_program.apr,
        expected.staked_assets.eden_boost_earn_program.apr
    );
    assert_eq!(
        resp.staked_assets.eden_boost_earn_program.available,
        expected.staked_assets.eden_boost_earn_program.available
    );
    assert_eq!(
        resp.staked_assets.eden_boost_earn_program.staked,
        expected.staked_assets.eden_boost_earn_program.staked
    );
    assert_eq!(
        resp.staked_assets.eden_boost_earn_program.rewards,
        expected.staked_assets.eden_boost_earn_program.rewards
    );
    assert_eq!(
        resp.staked_assets.eden_boost_earn_program,
        expected.staked_assets.eden_boost_earn_program
    );

    assert_eq!(resp.staked_assets, expected.staked_assets);

    assert_eq!(resp.total_staked_balance, expected.total_staked_balance);

    assert_eq!(resp, expected);
}