use std::str::FromStr as _;

use axelar_wasm_addresses::address;
use axelar_wasm_std::migrate_from_version;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Empty, Env, Response};
use multisig_prover_api::encoding::Encoder;
use router_api::ChainName;
use semver::{Version, VersionReq};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    state::{Config, CONFIG},
};

pub type MigrateMsg = Empty;

#[cfg_attr(not(feature = "library"), entry_point)]
#[migrate_from_version("1.1")]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, axelar_wasm_std::error::ContractError> {
    let old_version = Version::parse(&cw2::get_contract_version(deps.storage)?.version)?;
    let version_requirement = VersionReq::parse(">= 1.1.0, < 1.2.0")?;
    assert!(version_requirement.matches(&old_version));

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let old_config = CONFIG.load(deps.storage)?;
    let new_config = Config {
        chain_name: ChainName::from_str("aleo-2").unwrap(),
        encoder: Encoder::Aleo,
        key_type: multisig::key::KeyType::AleoSchnorr,
        multisig: address::validate_cosmwasm_address(
            deps.api,
            "axelar1g5vu3hs8g5hq3wy7q2p4c6q0aar08f3n2z73nrxgf56rg7yrzkds5kh89l",
        )?,
        ..old_config
    };

    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::default())
}
