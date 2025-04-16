use std::str::FromStr as _;

use aleo_gateway::{AleoValue, Message, Messages, PayloadDigest, WeightedSigners};
use axelar_wasm_std::hash::Hash;
use axelar_wasm_std::FnExt;
use cosmwasm_std::{HexBinary, Uint256};
use error_stack::{Result, ResultExt};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;
use snarkvm_cosmwasm::program::Network;

use crate::error::ContractError;
use crate::payload::Payload;

pub fn payload_digest<N: Network>(
    domain_separator: &Hash,
    verifier_set: &VerifierSet,
    payload: &Payload,
) -> Result<Hash, ContractError> {
    let data_hash = match payload {
        Payload::Messages(messages) => {
            let messages = messages
                .iter()
                .map(Message::try_from)
                .collect::<Result<Vec<_>, _>>()
                .change_context(ContractError::InvalidMessage)?
                .then(Messages::from);

            // TODO: make the MessageGroup configurable from outside
            let group: aleo_gateway::MessageGroup<N, 2, 2> =
                aleo_gateway::MessageGroup::new(messages.0)
                    .change_context(ContractError::InvalidMessage)?;

            group.bhp_string::<N>()
        }
        // TODO: make the WeightedSigners configurable from outside
        Payload::VerifierSet(verifier_set) => WeightedSigners::<2, 2>::try_from(verifier_set)
            .change_context(ContractError::InvalidVerifierSet)?
            .bhp_string::<N>(),
    }
    .map_err(|e| ContractError::AleoError(e.to_string()))?;

    let part1 = u128::from_le_bytes(domain_separator[0..16].try_into().map_err(|_| {
        ContractError::AleoError("Failed to convert domain separator to u128".to_string())
    })?);
    let part2 = u128::from_le_bytes(domain_separator[16..32].try_into().map_err(|_| {
        ContractError::AleoError("Failed to convert domain separator to u128".to_string())
    })?);
    let domain_separator: [u128; 2] = [part1, part2];

    let payload_digest = PayloadDigest::new(&domain_separator, verifier_set, data_hash)
        .map_err(|e| ContractError::AleoError(e.to_string()))?;

    let hash = payload_digest
        .bhp_string::<N>()
        .map_err(|e| ContractError::AleoError(e.to_string()))?;

    let next = hash.strip_suffix("group");
    let hash = next.unwrap_or(&hash);

    let hash =
        Uint256::from_str(hash).map_err(|e| error_stack::Report::new(ContractError::from(e)))?;
    let hash = hash.to_le_bytes();

    Ok(hash)
}

/// The relayer will use this data to submit the payload to the contract.
pub fn encode_execute_data(
    _domain_separator: &Hash,
    verifier_set: &VerifierSet,
    signatures: Vec<SignerWithSig>,
    payload: &Payload,
) -> Result<HexBinary, ContractError> {
    match payload {
        Payload::Messages(messages) => {
            let gmp_messages = messages
                .iter()
                .map(Message::try_from)
                .collect::<Result<Vec<_>, _>>()
                .change_context(ContractError::InvalidMessage)?
                .then(Messages::from);

            let proof = aleo_gateway::Proof::new(verifier_set.clone(), signatures)
                .change_context(ContractError::Proof)?;

            let execute_data = aleo_gateway::ExecuteDataMessages::new(proof, gmp_messages);

            let execute_data = execute_data
                .to_aleo_string()
                .map_err(|e| ContractError::AleoError(e.to_string()))?;

            Ok(HexBinary::from(execute_data.as_bytes()))
        }
        Payload::VerifierSet(verifier_set) => {
            let proof = aleo_gateway::Proof::new(verifier_set.clone(), signatures)
                .change_context(ContractError::Proof)?;

            let execute_data =
                aleo_gateway::ExecuteDataVerifierSet::<2, 2>::new(proof, verifier_set.clone())
                    .to_aleo_string()
                    .map_err(|e| ContractError::AleoError(e.to_string()))?;

            Ok(HexBinary::from(execute_data.as_bytes()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use axelar_wasm_std::Participant;
    use cosmwasm_std::Addr;
    use multisig::key::PublicKey;
    use multisig::msg::Signer;
    use router_api::ChainNameRaw;

    use super::*;

    fn message() -> router_api::Message {
        router_api::Message {
            cc_id: router_api::CrossChainId {
                source_chain: ChainNameRaw::from_str("aleo-2").unwrap(),
                message_id: "au1h9zxxrshyratfx0g0p5w8myqxk3ydfyxc948jysk0nxcna59ssqq0n3n3y"
                    .parse()
                    .unwrap(),
            },
            source_address: "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau"
                .parse()
                .unwrap(),
            destination_chain: "aleo-2".parse().unwrap(),
            destination_address: "foo".parse().unwrap(),
            payload_hash: [
                0xa4, 0x32, 0xdc, 0x98, 0x3d, 0xfe, 0x6f, 0xc4, 0x8b, 0xb4, 0x7a, 0x90, 0x91, 0x54,
                0x65, 0xd9, 0xc8, 0x18, 0x5b, 0x1c, 0x2a, 0xea, 0x5c, 0x87, 0xf8, 0x58, 0x18, 0xcb,
                0xa3, 0x50, 0x51, 0xc6,
            ],
        }
    }

    type Curr = snarkvm::prelude::TestnetV0;

    use tofn::aleo_schnorr::keygen;
    use tofn::sdk::api::SecretRecoveryKey;

    // APrivateKey1zkpFMDCJZbRdcBcjnqjRqCrhcWFf4L9FRRSgbLpS6D47Cmo
    // aleo1v7mmux8wkue8zmuxdfks03rh85qchfmms9fkpflgs4dt87n4jy9s8nzfss
    fn aleo_sig(digest: [u8; 32]) -> SignerWithSig {
        let arr = [0; 64];
        let k = SecretRecoveryKey::try_from(&arr[..]).unwrap();
        let key_pair = keygen::<Curr>(&k, b"tofn nonce").unwrap();
        let msg = tofn::sdk::api::MessageDigest::from(digest);
        let signature = tofn::aleo_schnorr::sign(&key_pair, &msg).unwrap();

        let _signature_str = String::from_utf8(signature.clone()).unwrap();
        let verify_key = key_pair.encoded_verifying_key();

        let signer = Signer {
            address: Addr::unchecked("aleo-validator".to_string()),
            weight: 1u128.into(),
            pub_key: PublicKey::AleoSchnorr(HexBinary::from(verify_key.as_bytes())),
        };

        let signature = multisig::key::Signature::AleoSchnorr(HexBinary::from(&signature[..]));

        SignerWithSig { signer, signature }
    }

    use std::convert::TryFrom;

    #[test]
    fn aleo_execute_data() {
        let domain_separator = [
            105u8, 115u8, 199u8, 41u8, 53u8, 96u8, 68u8, 100u8, 178u8, 136u8, 39u8, 20u8, 27u8,
            10u8, 70u8, 58u8, 248u8, 227u8, 72u8, 118u8, 22u8, 222u8, 105u8, 197u8, 170u8, 12u8,
            120u8, 83u8, 146u8, 201u8, 251u8, 159u8,
        ];

        let verifier_set = VerifierSet::new(
            vec![
                (Participant {
                    address: Addr::unchecked("axelar1ckguw8l9peg6sykx30cy35t6l0wpfu23xycme7"),
                    weight: 1.try_into().unwrap(),
                },
                PublicKey::AleoSchnorr(HexBinary::from(hex::decode("616C656F3176376D6D757838776B7565387A6D757864666B73303372683835716368666D6D7339666B70666C677334647438376E346A793973386E7A667373").unwrap())),
                )],
            1u128.into(),
            4860541,
        );

        let digest = payload_digest::<snarkvm_cosmwasm::network::TestnetV0>(
            &domain_separator,
            &verifier_set,
            &Payload::Messages(vec![message()]),
        )
        .unwrap();

        let _execute_data = encode_execute_data(
            &domain_separator,
            &verifier_set,
            vec![aleo_sig(digest)],
            &Payload::Messages(vec![message()]),
        )
        .unwrap();
    }
}
