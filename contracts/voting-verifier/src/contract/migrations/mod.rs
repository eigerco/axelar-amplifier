use axelar_wasm_addresses::address;
use axelar_wasm_std::{migrate_from_version, nonempty::Uint64, MajorityThreshold, Threshold};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Empty, Env, Response};

use crate::{contract::{CONTRACT_NAME, CONTRACT_VERSION}, state::{Config, CONFIG}};

pub type MigrateMsg = Empty;

#[cfg_attr(not(feature = "library"), entry_point)]
#[migrate_from_version("1.1")]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, axelar_wasm_std::error::ContractError> {
    // TODO: THIS FUNCTION SHOULD BE REVERTED, AND THE CODE ADDED BELOW SHOULD BE DELETED BEFORE MERGING TO AXELAR-AMPLIFIER
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        service_name: "validators".parse().unwrap(),
        service_registry_contract: cosmwasm_std::Addr::unchecked(
            "axelar1c9fkszt5lq34vvvlat3fxj6yv7ejtqapz04e97vtc9m5z9cwnamq8zjlhz",
        ),
        source_gateway_address: "vzevxifdoj.aleo".parse().unwrap(),
        voting_threshold: MajorityThreshold::try_from(Threshold::try_from((1, 1)).unwrap())
            .unwrap(),
        block_expiry: Uint64::try_from(10u64).unwrap(),
        confirmation_height: 1,
        source_chain: "aleo-2".parse().unwrap(),
        rewards_contract: cosmwasm_std::Addr::unchecked(
            "axelar1vaj9sfzc3z0gpel90wu4ljutncutv0wuhvvwfsh30rqxq422z89qnd989l",
        ),
        msg_id_format: axelar_wasm_std::msg_id::MessageIdFormat::Bech32m {
            prefix: "au".to_string().try_into().unwrap(),
            length: 61,
        },
        address_format: address::AddressFormat::Aleo,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}
