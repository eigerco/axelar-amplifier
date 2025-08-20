use cosmwasm_std::HexBinary;
use starknet_crypto::Felt;

use crate::error::ContractError;

pub fn verify_signature(
    signature: HexBinary,
    message: HexBinary,
    public_key: HexBinary,
) -> Result<bool, ContractError> {
    let signature_bytes = signature.as_slice();
    if signature_bytes.len() != 96 {
        return Err(ContractError::InvalidSignatureLength(signature_bytes.len()));
    }

    let (r, s, _v) = parse_signature_components(&signature)?;
    let message_felt = hex_to_felt(&message)?;
    let public_key_felt = hex_to_felt(&public_key)?;

    match starknet_crypto::verify(&public_key_felt, &message_felt, &r, &s) {
        Ok(is_valid) => Ok(is_valid),
        Err(_) => Err(ContractError::VerificationFailed),
    }
}

fn hex_to_felt(hex_binary: &HexBinary) -> Result<Felt, ContractError> {
    let bytes = hex_binary.as_slice();
    if bytes.len() != 32 {
        return Err(ContractError::InvalidMessage(format!(
            "Expected 32 bytes, got {} bytes",
            bytes.len()
        )));
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(bytes);

    Ok(Felt::from_bytes_be(&array))
}

fn parse_signature_components(signature: &HexBinary) -> Result<(Felt, Felt, Felt), ContractError> {
    let bytes = signature.as_slice();

    let r_bytes = &bytes[0..32];
    let s_bytes = &bytes[32..64];
    let v_bytes = &bytes[64..96];

    let r = felt_from_bytes(r_bytes)?;
    let s = felt_from_bytes(s_bytes)?;
    let v = felt_from_bytes(v_bytes)?;

    Ok((r, s, v))
}

fn felt_from_bytes(bytes: &[u8]) -> Result<Felt, ContractError> {
    let mut array = [0u8; 32];
    array.copy_from_slice(bytes);

    Ok(Felt::from_bytes_be(&array))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::HexBinary;
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
    fn test_verify_signature_success() {
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

        let signature_hex = HexBinary::from(signature_bytes);
        let message_hex = felt_to_hex_binary(&message);
        let public_key_hex = felt_to_hex_binary(&public_key);

        let result = verify_signature(signature_hex, message_hex, public_key_hex).unwrap();
        assert!(result, "Valid signature should verify successfully");
    }

    #[test]
    fn test_verify_signature_invalid_signature() {
        let (_, public_key) = create_test_key_pair();
        let message =
            Felt::from_hex("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .unwrap();

        let mut invalid_signature_bytes = vec![0u8; 96];
        invalid_signature_bytes[0] = 0xFF;

        let signature_hex = HexBinary::from(invalid_signature_bytes);
        let message_hex = felt_to_hex_binary(&message);
        let public_key_hex = felt_to_hex_binary(&public_key);

        let result = verify_signature(signature_hex, message_hex, public_key_hex);
        match result {
            Ok(false) => {}                              // Invalid signature should return false
            Err(ContractError::VerificationFailed) => {} // Or it might return VerificationFailed error
            _ => panic!("Expected either Ok(false) or VerificationFailed error"),
        }
    }

    #[test]
    fn test_verify_signature_wrong_length() {
        let signature_hex = HexBinary::from(vec![0u8; 95]);
        let message_hex = HexBinary::from(vec![0u8; 32]);
        let public_key_hex = HexBinary::from(vec![0u8; 32]);

        let result = verify_signature(signature_hex, message_hex, public_key_hex);
        match result {
            Err(ContractError::InvalidSignatureLength(95)) => {}
            _ => panic!("Expected InvalidSignatureLength error"),
        }
    }

    #[test]
    fn test_verify_signature_invalid_message_length() {
        let signature_hex = HexBinary::from(vec![0u8; 96]);
        let message_hex = HexBinary::from(vec![0u8; 31]);
        let public_key_hex = HexBinary::from(vec![0u8; 32]);

        let result = verify_signature(signature_hex, message_hex, public_key_hex);
        match result {
            Err(ContractError::InvalidMessage(_)) => {}
            _ => panic!("Expected InvalidMessage error"),
        }
    }

    #[test]
    fn test_verify_signature_invalid_public_key_length() {
        let signature_hex = HexBinary::from(vec![0u8; 96]);
        let message_hex = HexBinary::from(vec![0u8; 32]);
        let public_key_hex = HexBinary::from(vec![0u8; 31]);

        let result = verify_signature(signature_hex, message_hex, public_key_hex);
        match result {
            Err(ContractError::InvalidMessage(_)) => {}
            _ => panic!("Expected InvalidMessage error for public key length"),
        }
    }

    #[test]
    fn test_hex_to_felt_valid() {
        let bytes = vec![
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xab, 0xcd, 0xef,
        ];
        let hex_binary = HexBinary::from(bytes);
        let result = hex_to_felt(&hex_binary);
        assert!(result.is_ok(), "Valid 32-byte input should convert to Felt");
    }

    #[test]
    fn test_hex_to_felt_invalid_length() {
        let hex_binary = HexBinary::from(vec![0u8; 31]);
        let result = hex_to_felt(&hex_binary);
        match result {
            Err(ContractError::InvalidMessage(_)) => {}
            _ => panic!("Expected InvalidMessage error for wrong length"),
        }
    }

    #[test]
    fn test_parse_signature_components_valid() {
        let r_bytes = vec![0x01; 32];
        let s_bytes = vec![0x02; 32];
        let v_bytes = vec![0x03; 32];

        let mut signature_bytes = Vec::new();
        signature_bytes.extend_from_slice(&r_bytes);
        signature_bytes.extend_from_slice(&s_bytes);
        signature_bytes.extend_from_slice(&v_bytes);

        let signature_hex = HexBinary::from(signature_bytes);
        let result = parse_signature_components(&signature_hex);
        assert!(
            result.is_ok(),
            "Valid signature bytes should parse successfully"
        );
    }
}
