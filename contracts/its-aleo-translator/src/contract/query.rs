use cosmwasm_std::{Binary, Deps, Env, HexBinary};
use interchain_token_service_std::HubMessage;

use crate::aleo::{aleo_inbound_hub_message, aleo_outbound_hub_message};
use crate::error::ContractError;

pub fn from_bytes(_deps: Deps, _env: Env, payload: HexBinary) -> Result<Binary, ContractError> {
    let hub_message = aleo_outbound_hub_message::<snarkvm_cosmwasm::prelude::TestnetV0>(payload)
        .map_err(|_| ContractError::SerializationFailed)?;
    cosmwasm_std::to_json_binary(&hub_message).map_err(|_| ContractError::SerializationFailed)
}

pub fn to_bytes(_deps: Deps, _env: Env, message: HubMessage) -> Result<Binary, ContractError> {
    let payload = aleo_inbound_hub_message::<snarkvm_cosmwasm::prelude::TestnetV0>(message)
        .map_err(|_| ContractError::SerializationFailed)?;
    cosmwasm_std::to_json_binary(&payload).map_err(|_| ContractError::SerializationFailed)
}
