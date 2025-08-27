use starknet_core::types::Felt;
use starknet_crypto::{verify, ExtendedSignature};

use crate::ContractError;

// STARK signatures are 96 bytes: r (32) + s (32) + v (32)
pub const STARK_SIGNATURE_LEN: usize = 96;

pub fn stark_verify(msg_hash: &[u8], sig: &[u8], pub_key: &[u8]) -> Result<bool, ContractError> {
    if sig.len() != STARK_SIGNATURE_LEN {
        return Err(ContractError::SignatureVerificationFailed {
            reason: format!(
                "Invalid signature length: expected {}, got {}",
                STARK_SIGNATURE_LEN,
                sig.len()
            ),
        });
    }

    let msg_felt = Felt::from_bytes_be_slice(msg_hash);
    let pub_key_felt = Felt::from_bytes_be_slice(pub_key);
    let r = Felt::from_bytes_be_slice(&sig[0..32]);
    let s = Felt::from_bytes_be_slice(&sig[32..64]);
    let v = Felt::from_bytes_be_slice(&sig[64..96]);

    let extended_sig = ExtendedSignature { r, s, v };

    match verify(&pub_key_felt, &msg_felt, &extended_sig.r, &extended_sig.s) {
        Ok(result) => Ok(result),
        Err(e) => Err(ContractError::SignatureVerificationFailed {
            reason: format!("STARK signature verification failed: {}", e),
        }),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::HexBinary;

    use super::*;
    use crate::test::common::stark_test_data;

    #[test]
    fn should_fail_sig_verification_instead_of_truncating() {
        let sig_with_extra_byte = stark_test_data::signature().to_hex() + "00";

        let signature = HexBinary::from_hex(&sig_with_extra_byte).unwrap().to_vec();
        let message = stark_test_data::message().to_vec();
        let public_key = stark_test_data::pub_key().to_vec();

        let result = stark_verify(&message, &signature, &public_key);
        assert_eq!(
            result.unwrap_err(),
            ContractError::SignatureVerificationFailed {
                reason: format!(
                    "Invalid signature length: expected {}, got {}",
                    STARK_SIGNATURE_LEN,
                    signature.len()
                ),
            }
        );
    }
}
