use crate::{error::ContractError, payload::Payload};
use axelar_wasm_std::{hash::Hash, operators::Operators};
use multisig::worker_set::WorkerSet;

pub mod execute_data;

pub fn make_operators(_worker_set: WorkerSet) -> Operators {
    todo!()
}

pub fn payload_hash_to_sign(
    _domain_separator: &Hash,
    _signer: &WorkerSet,
    _payload: &Payload,
) -> Result<Hash, ContractError> {
    todo!()
}

pub fn encode(_payload: &Payload) -> Result<Vec<u8>, ContractError> {
    todo!()
}
