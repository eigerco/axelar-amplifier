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

            messages
                .0
                .first()
                .ok_or(ContractError::InvalidMessage)?
                .bhp::<N>()
        }
        Payload::VerifierSet(verifier_set) => WeightedSigners::try_from(verifier_set)
            .change_context(ContractError::InvalidVerifierSet)?
            .bhp::<N>(),
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
        .bhp::<N>()
        .map_err(|e| ContractError::AleoError(e.to_string()))?;

    let next = hash.strip_suffix("group");
    let hash = next.unwrap_or(&hash);

    let hash = Uint256::from_str(&hash).unwrap();
    let hash = hash.to_le_bytes();

    Ok(hash)
}

/// The relayer will use this data to submit the payload to the contract.
pub fn encode_execute_data(
    domain_separator: &Hash,
    verifier_set: &VerifierSet,
    signatures: Vec<SignerWithSig>,
    payload: &Payload,
) -> Result<HexBinary, ContractError> {
    let payload = match payload {
        Payload::Messages(messages) => messages
            .iter()
            .map(Message::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(ContractError::InvalidMessage)?
            .then(Messages::from),
        Payload::VerifierSet(verifier_set) => todo!(),
    };

    let proof = aleo_gateway::Proof::new(
        verifier_set.clone(),
        signatures.first().ok_or(ContractError::Proof)?.clone(),
    )
    .change_context(ContractError::Proof)?;

    let execute_data = aleo_gateway::ExecuteData::new(
        proof,
        payload.0.first().ok_or(ContractError::Proof)?.clone(),
    );

    let execute_data = execute_data
        .to_aleo_string()
        .map_err(|e| ContractError::AleoError(e.to_string()))?;

    Ok(HexBinary::from(execute_data.as_bytes()))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use axelar_wasm_std::Participant;
    use cosmwasm_std::Addr;
    use multisig::key::PublicKey;
    use router_api::{ChainName, ChainNameRaw};

    use super::*;

    #[test]
    fn aleo_digest() {
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
                PublicKey::AleoSchnorr(HexBinary::from(hex::decode("616c656f313435746a396871726e76336871796c72656d3670377a6a797863326b727979703368646d34687434386e746a336535747475787339787339616b").unwrap())),
                )],
            1u128.try_into().unwrap(),
            4860541,
        );

        // "value": "{\"messages\":[{\"cc_id\":{\"source_chain\":\"aleo-2\",\"message_id\":\"au1h9zxxrshyratfx0g0p5w8myqxk3ydfyxc948jysk0nxcna59ssqq0n3n3y\"},
        // \"source_address\":\"aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau\",
        // \"destination_chain\":\"aleo-2\",\"destination_address\":\"666f6f0000000000000000000000000000000000\",\"payload_hash\":\"a432dc983dfe6fc48bb47a90915465d9c8185b1c2aea5c87f85818cba35051c6\"}]}"

        let message = router_api::Message {
            cc_id: router_api::CrossChainId {
                source_chain: ChainNameRaw::from_str("aleo-2").unwrap(),
                message_id: "au1h9zxxrshyratfx0g0p5w8myqxk3ydfyxc948jysk0nxcna59ssqq0n3n3y"
                    .parse()
                    .unwrap(),
            },
            source_address: "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau"
                .parse()
                .unwrap(),
            // .to_string(),
            destination_chain: "aleo-2".parse().unwrap(),
            destination_address: "666f6f0000000000000000000000000000000000".parse().unwrap(), // to_string(),
            payload_hash: [
                0xa4, 0x32, 0xdc, 0x98, 0x3d, 0xfe, 0x6f, 0xc4, 0x8b, 0xb4, 0x7a, 0x90, 0x91, 0x54,
                0x65, 0xd9, 0xc8, 0x18, 0x5b, 0x1c, 0x2a, 0xea, 0x5c, 0x87, 0xf8, 0x58, 0x18, 0xcb,
                0xa3, 0x50, 0x51, 0xc6,
            ],
        };

        // The hash of the payload, send from the relayer of the source chain
        let payload = Payload::Messages(vec![message]);

        let res = payload_digest::<snarkvm_cosmwasm::network::TestnetV0>(
            &domain_separator,
            &verifier_set,
            &payload,
        );
        println!("{:?}", res);
    }
}
