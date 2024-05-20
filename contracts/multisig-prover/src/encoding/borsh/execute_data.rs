use crate::{error::ContractError, payload::Payload};
use axelar_wasm_std::hash::Hash;
use cosmwasm_std::HexBinary;
use multisig::{msg::SignerWithSig, worker_set::WorkerSet};

pub fn encode(
    _worker_set: &WorkerSet,
    _signers: Vec<SignerWithSig>,
    _payload_digest: &Hash,
    _payload: &Payload,
) -> Result<HexBinary, ContractError> {
    todo!()
}
