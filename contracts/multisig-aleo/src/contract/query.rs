use std::str::FromStr as _;

use cosmwasm_std::{HexBinary, StdResult};
use snarkvm_cosmwasm::account::ToFields;
use snarkvm_cosmwasm::program::{Network, Signature, Value};
use snarkvm_cosmwasm::types::{Address, Field};

pub fn verify_signature<N: Network>(
    signature: HexBinary,
    message: HexBinary,
    public_key: HexBinary,
) -> StdResult<bool> {
    let signed = String::from_utf8(signature.into()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
    })?;

    let signature = Signature::<N>::from_str(signed.as_str()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
    })?;

    let address = String::from_utf8(public_key.into()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse public key: {}", e))
    })?;

    let addr = Address::from_str(address.as_str()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse address: {}", e))
    })?;

    let message = aleo_encoded(&message)?;

    let res = signature.verify(&addr, message.as_slice());
    Ok(res)
}

fn aleo_encoded<N: Network>(data: &HexBinary) -> Result<Vec<Field<N>>, cosmwasm_std::StdError> {
    let num = cosmwasm_std::Uint256::from_le_bytes(data.as_slice().try_into().unwrap());
    let message = format!("{num}group");

    Value::from_str(message.as_str())
        .map_err(|e| {
            cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
        })?
        .to_fields()
        .map_err(|e| {
            cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature() {
        let msg = "6639d4b27d2c040accec47fb480ce774194604b30e5ce1cf395ff0c63af87c0f";
        let signature = "7369676e317a6836657178677436686d61746a6c3378776b6c64367639706b687178783567647774776e3677726e686673686163377873713377366164747a7870676b6b613239716d32356175796568636e6c637063777a3972736b686a38666732373735367434707171757078703661706372676e7a61773367666479776c61396d347678797776686e73756577643338616c7377703370786d75387a7436706a35346e673877327478766e7a6a70356333707975396c7435346636686c6778756c6e39386a677a72776e70737161737333763778746e";
        let address = "aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak";

        let msg = HexBinary::from_hex(msg).unwrap();
        let signature = HexBinary::from_hex(signature).unwrap();
        let address = HexBinary::from(address.as_bytes());

        assert_eq!(
            verify_signature::<snarkvm_cosmwasm::network::TestnetV0>(signature, msg, address),
            Ok(true)
        );
    }
}
