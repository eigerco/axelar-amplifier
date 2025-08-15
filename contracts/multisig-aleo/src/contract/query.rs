use cosmwasm_std::HexBinary;
use snarkvm_cosmwasm::prelude::{Address, FromBytes, Network, Signature};

use crate::ContractError;

pub fn verify_signature<N: Network>(
    signature: HexBinary,
    message: HexBinary,
    public_key: HexBinary,
) -> Result<bool, ContractError> {
    let signature = Signature::<N>::from_bytes_le(&signature)?;

    let address = Address::<N>::from_bytes_le(&public_key)?;

    let res = signature.verify_bytes(&address, &message);

    Ok(res)
}

#[cfg(test)]
mod tests {
    use snarkvm_cosmwasm::console::network::TestnetV0;
    use tofn::aleo_schnorr::{sign, KeyPair};

    use super::*;

    type CurrentNetwork = TestnetV0;

    #[test]
    fn test_verify_signature() {
        let message = [
            30, 165, 51, 99, 240, 22, 44, 209, 224, 46, 25, 4, 49, 49, 114, 238, 209, 48, 186, 136,
            95, 224, 128, 254, 19, 109, 54, 40, 214, 206, 187, 13,
        ]
        .into();

        let key_pair: KeyPair<CurrentNetwork> =
            tofn::aleo_schnorr::dummy_keygen().expect("Failed to generate key pair");
        let encoded_signature = sign(&key_pair, &message).expect("Failed to sign message");

        let signature = HexBinary::from(encoded_signature);
        let address = HexBinary::from(
            key_pair
                .encoded_verifying_key()
                .expect("Failed to get verifying key"),
        );
        let message = HexBinary::from(message.as_ref());

        assert!(
            verify_signature::<CurrentNetwork>(signature, message, address)
                .expect("Failed to verify signature"),
        );
    }
}
