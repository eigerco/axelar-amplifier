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

    Ok(payload_digest
        .hash()
        .map_err(|e| ContractError::AleoError(e.to_string()))?)
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

    let proof = aleo_gateway::Proof::new(verifier_set.clone(), signatures)
        .change_context(ContractError::Proof)?;

    let execute_data = aleo_gateway::ExecuteData::new(proof, payload);
    let execute_data = execute_data
        .to_aleo_string()
        .change_context(ContractError::SerializeData)?;
    Ok(HexBinary::from(execute_data.as_bytes()))
}
