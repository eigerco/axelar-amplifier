use std::str::FromStr as _;

use cosmwasm_std::{HexBinary, StdResult};

pub fn verify_signature(
    signature: HexBinary,
    message: HexBinary,
    public_key: HexBinary,
) -> StdResult<bool> {
    // TODO: make network generic
    type Curr = snarkvm_wasm::network::TestnetV0;

    let signed = String::from_utf8(signature.into()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
    })?;

    let signature: snarkvm_wasm::account::signature::Signature<Curr> =
        snarkvm_wasm::account::signature::Signature::from_str(signed.as_str()).map_err(|e| {
            cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
        })?;

    let address = String::from_utf8(public_key.into()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse public key: {}", e))
    })?;

    let addr = snarkvm_wasm::types::Address::<Curr>::from_str(address.as_str()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse address: {}", e))
    })?;

    let res = signature.verify_bytes(&addr, message.as_slice());
    Ok(res)
}
