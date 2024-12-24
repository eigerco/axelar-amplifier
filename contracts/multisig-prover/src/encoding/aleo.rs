use aleo_gateway::{Message, Messages, PayloadDigest, WeightedSigners};
use axelar_wasm_std::hash::Hash;
use axelar_wasm_std::FnExt;
use cosmwasm_std::HexBinary;
use error_stack::{Result, ResultExt};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;

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
            .hash(),
        Payload::VerifierSet(verifier_set) => WeightedSigners::try_from(verifier_set)
            .change_context(ContractError::InvalidVerifierSet)?
            .hash(),
    }
    .change_context(ContractError::SerializeData)?;

    let signers_hash = WeightedSigners::try_from(verifier_set)
        .change_context(ContractError::InvalidVerifierSet)?
        .hash()
        .change_context(ContractError::SerializeData)?;

    let payload_digest = PayloadDigest::new(domain_separator, &signers_hash, &data_hash);

    Ok(payload_digest.hash())
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
            .then(Messages::from)
            .then(|m| m.to_aleo_string()),
        Payload::VerifierSet(verifier_set) => WeightedSigners::try_from(verifier_set)
            .change_context(ContractError::InvalidVerifierSet)?
            .then(|v| v.to_aleo_string()),
    };

    todo!()
}
