use cosmwasm_schema::cw_serde;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, DepsMut, Empty, Env, MessageInfo, Response};
use signature_verifier_api::msg::ExecuteMsg;

use crate::error::ContractError;

#[cw_serde]
pub struct InstantiateMsg {}

pub mod execute;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _msg: Empty,
) -> Result<Response, axelar_wasm_std::error::ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, axelar_wasm_std::error::ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let result = match msg {
        ExecuteMsg::VerifySignature {
            signature,
            message,
            public_key,
            signer_address: _,
            session_id: _,
        } => execute::verify_signature(signature, message, public_key)?,
    };

    Ok(Response::new().set_data(to_json_binary(&result)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{from_json, Addr, HexBinary, Uint64};
    use starknet_crypto::Felt;

    fn create_test_key_pair() -> (Felt, Felt) {
        let private_key =
            Felt::from_hex("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
                .unwrap();
        let public_key = starknet_crypto::get_public_key(&private_key);
        (private_key, public_key)
    }

    fn felt_to_hex_binary(felt: &Felt) -> HexBinary {
        let bytes = felt.to_bytes_be();
        HexBinary::from(bytes.to_vec())
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let msg = InstantiateMsg {};

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
    }

    #[test]
    fn test_execute_verify_signature_success() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = message_info(&Addr::unchecked("sender"), &[]);

        let (private_key, public_key) = create_test_key_pair();
        let message =
            Felt::from_hex("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .unwrap();

        // Generate k value for deterministic signature
        let k = starknet_crypto::rfc6979_generate_k(&message, &private_key, None);
        let signature = starknet_crypto::sign(&private_key, &message, &k).unwrap();

        let mut signature_bytes = Vec::new();
        signature_bytes.extend_from_slice(&signature.r.to_bytes_be());
        signature_bytes.extend_from_slice(&signature.s.to_bytes_be());
        signature_bytes.extend_from_slice(&signature.v.to_bytes_be());

        let msg = ExecuteMsg::VerifySignature {
            signature: HexBinary::from(signature_bytes),
            message: felt_to_hex_binary(&message),
            public_key: felt_to_hex_binary(&public_key),
            signer_address: "test_signer".to_string(),
            session_id: Uint64::new(1),
        };

        let response = execute(deps.as_mut(), env, info, msg).unwrap();
        let result: bool = from_json(&response.data.unwrap()).unwrap();
        assert!(result, "Valid signature should verify successfully");
    }

    #[test]
    fn test_execute_verify_signature_invalid() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = message_info(&Addr::unchecked("sender"), &[]);

        let (_, public_key) = create_test_key_pair();
        let message =
            Felt::from_hex("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .unwrap();

        let mut invalid_signature_bytes = vec![0u8; 96];
        invalid_signature_bytes[0] = 0xFF;

        let msg = ExecuteMsg::VerifySignature {
            signature: HexBinary::from(invalid_signature_bytes),
            message: felt_to_hex_binary(&message),
            public_key: felt_to_hex_binary(&public_key),
            signer_address: "test_signer".to_string(),
            session_id: Uint64::new(1),
        };

        let result = execute(deps.as_mut(), env, info, msg);
        match result {
            Ok(response) => {
                let result: bool = from_json(&response.data.unwrap()).unwrap();
                assert!(!result, "Invalid signature should return false");
            }
            Err(ContractError::VerificationFailed) => {
                // This is also acceptable - invalid signature can return an error
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_execute_verify_signature_wrong_length() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = message_info(&Addr::unchecked("sender"), &[]);

        let msg = ExecuteMsg::VerifySignature {
            signature: HexBinary::from(vec![0u8; 95]),
            message: HexBinary::from(vec![0u8; 32]),
            public_key: HexBinary::from(vec![0u8; 32]),
            signer_address: "test_signer".to_string(),
            session_id: Uint64::new(1),
        };

        let result = execute(deps.as_mut(), env, info, msg);
        match result {
            Err(ContractError::InvalidSignatureLength(95)) => {}
            _ => panic!("Expected InvalidSignatureLength error"),
        }
    }
}
