use std::str::FromStr as _;

use aleo_gateway::{Message, Messages, Proof, WeightedSigners};
use axelar_wasm_std::hash::Hash;
use axelar_wasm_std::FnExt;
use cosmwasm_std::HexBinary;
use error_stack::{Result, ResultExt};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;
use sha3::{Digest, Keccak256};

use crate::error::ContractError;
use crate::payload::Payload;

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
            .messages_approval_hash(),
        Payload::VerifierSet(verifier_set) => todo!(),
        /*
        WeightedSigners::try_from(verifier_set)
                    .change_context(ContractError::InvalidVerifierSet)?
                    .signers_rotation_hash()
        */
    }
    .change_context(ContractError::SerializeData)?;
    todo!()
    // let signers_hash = WeightedSigners::try_from(verifier_set)
    //     .change_context(ContractError::InvalidVerifierSet)?
    //     .hash()
    //     .change_context(ContractError::SerializeData)?;
    //
    // let unsigned = [
    //     domain_separator,
    //     signers_hash.as_slice(),
    //     data_hash.as_slice(),
    // ]
    // .concat();
    //
    // Ok(Keccak256::digest(unsigned).into())
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
