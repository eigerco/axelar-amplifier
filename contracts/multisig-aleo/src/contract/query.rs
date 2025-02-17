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
        let msg = "df4fd7e608879cb128c53f82614cdffcfd163b4d14adbe5c797f3aaaa3e316b8";
        let signature = "7369676e317179713530387371793264327a706a6a3364676b616d636379377272307a30777175656e3966346e6377367874396c326576717a6b646a6d38716b6d796c7038636a6675716434387937656a657937676b777a6d66787039756868376366636c6c7034666370797078703661706372676e7a61773367666479776c61396d347678797776686e73756577643338616c7377703370786d75387a7436706a35346e673877327478766e7a6a70356333707975396c7435346636686c6778756c6e39386a677a72776e70737161737364743376346e";
        let address = "aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak";

        let msg = HexBinary::from_hex(msg).unwrap();
        let signature = HexBinary::from_hex(signature).unwrap();
        let address = HexBinary::from(address.as_bytes());

        assert_eq!(
            verify_signature::<snarkvm_cosmwasm::network::TestnetV0>(signature, msg, address),
            Ok(true)
        );
    }

    #[test]
    fn test_2() {
        let msg = "a784e1c72d1ed2090c38dbd25304d47a8e86284be45325d97a8ad181f6bd3700";
        let signature = "7369676e31756633676d32727170307464673333706330357265703265387a39706e74793574706e6777327071616e7a79723270367935703465656c657161667439733433676d346471356a66336e6761673865307a7267757875646735346b6a7336376d767676636371767078703661706372676e7a61773367666479776c61396d347678797776686e73756577643338616c7377703370786d75387a7436706a35346e673877327478766e7a6a70356333707975396c7435346636686c6778756c6e39386a677a72776e707371617373346c746a3633";
        let address = "aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak";

        let msg = HexBinary::from_hex(msg).unwrap();
        let signature = HexBinary::from_hex(signature).unwrap();
        let address = HexBinary::from(address.as_bytes());

        assert_eq!(
            verify_signature::<snarkvm_cosmwasm::network::TestnetV0>(signature, msg, address),
            Ok(true)
        );
    }

    #[test]
    fn test_3() {
        let msg = "b02ac668d2c8f5d3f5d95834ca11ef5244314451602b8faa9e49a6ac5ec5360a";
        let signature = "7369676e316b6e3437366175676166336c716d756675637a357a756136676a7974746439366e76346b72366873673679793832366d666770706d347877727972306b307230323973787773673774756e7a39706834736e3278333577733937766e66757238753978327a71357078703661706372676e7a61773367666479776c61396d347678797776686e73756577643338616c7377703370786d75387a7436706a35346e673877327478766e7a6a70356333707975396c7435346636686c6778756c6e39386a677a72776e70737161737374716b306879";
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
