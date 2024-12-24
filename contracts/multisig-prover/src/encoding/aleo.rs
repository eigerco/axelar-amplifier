use std::str::FromStr as _;

use aleo_gateway::{Error, Message, Messages, Proof};
use axelar_wasm_std::hash::Hash;
use axelar_wasm_std::FnExt;
use cosmwasm_std::HexBinary;
use error_stack::{Report, Result, ResultExt};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;
use sha3::{Digest, Keccak256};
use snarkvm_wasm::program::ToBits;

use crate::error::ContractError;
use crate::payload::Payload;

// use snarkvm_wasm::snarkvm_console_network::network::Network;
use snarkvm_wasm::network::Network;

pub fn payload_digest(
    domain_separator: &Hash,
    verifier_set: &VerifierSet,
    payload: &Payload,
) -> Result<Hash, ContractError> {
    let data_hash = match payload {
        Payload::Messages(messages) => messages
            .iter()
            .map(Message::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(ContractError::InvalidMessage)?
            .then(Messages::from)
            .hash(),
        Payload::VerifierSet(verifier_set) => todo!(),
        /*
        WeightedSigners::try_from(verifier_set)
                    .change_context(ContractError::InvalidVerifierSet)?
                    .signers_rotation_hash()
        */
    }
    .change_context(ContractError::SerializeData)?;

    let (address, signer) = verifier_set.signers.iter().next().unwrap();
    let signer = match &signer.pub_key {
        multisig::key::PublicKey::AleoSchnorr(key) => {
            let key = key.to_string();
            let key = snarkvm_wasm::types::address::Address::from_str(&key).unwrap();
            Ok(key)
        }
        multisig::key::PublicKey::Ed25519(_) => Err(Report::new(Error::UnsupportedPublicKey)),
        multisig::key::PublicKey::Ecdsa(_) => Err(Report::new(Error::UnsupportedPublicKey)),
    };
    let signer: snarkvm_wasm::types::Address<snarkvm_wasm::network::TestnetV0> = signer.unwrap();

    let msg = "some message";

    let aleo_value =
        snarkvm_wasm::program::Value::<snarkvm_wasm::network::TestnetV0>::from_str(msg).unwrap();
    let aleo_value = aleo_value.to_bits_le();

    let aleo_group = snarkvm_wasm::network::TestnetV0::hash_to_group_bhp256(&aleo_value).unwrap();

    let signer_aleo_string = format!("signer: {}", signer.to_string());

    let threshold = verifier_set.threshold;
    let nonce = [0, 0, 0, verifier_set.created_at];

    // let unsigned = [
    //     domain_separator,
    //     signers_hash.as_slice(),
    //     data_hash.as_slice(),
    // ]
    // .concat();

    todo!()
}

// fn encode_payload(payload: )

/// `encode_execute_data` returns the XDR encoded external gateway function call args.
/// The relayer will use this data to submit the payload to the contract.
pub fn encode_execute_data(
    domain_separator: &Hash,
    verifier_set: &VerifierSet,
    signatures: Vec<SignerWithSig>,
    payload: &Payload,
) -> Result<HexBinary, ContractError> {
    todo!()
    // let payload = match payload {
    //     Payload::Messages(messages) => ScVal::try_from(
    //         messages
    //             .iter()
    //             .map(Message::try_from)
    //             .collect::<Result<Vec<_>, _>>()
    //             .change_context(ContractError::InvalidMessage)?
    //             .then(Messages::from),
    //     ),
    //     Payload::VerifierSet(verifier_set) => ScVal::try_from(
    //         WeightedSigners::try_from(verifier_set)
    //             .change_context(ContractError::InvalidVerifierSet)?,
    //     ),
    // }
    // .change_context(ContractError::SerializeData)?;
    //
    // let proof =
    //     Proof::try_from((verifier_set.clone(), signatures)).change_context(ContractError::Proof)?;
    //
    // let execute_data = ScVal::try_from((payload, proof))
    //     .expect("must convert tuple of size 2 to ScVec")
    //     .to_xdr(Limits::none())
    //     .change_context(ContractError::SerializeData)?;
    //
    // Ok(execute_data.as_slice().into())
}
