use crate::{error::ContractError, payload::Payload};
use axelar_wasm_std::{hash::Hash, operators::Operators};
use multisig::worker_set::WorkerSet;

pub mod execute_data;

pub fn make_operators(worker_set: WorkerSet) -> Operators {
    use multisig::key::PublicKey::*;
    let mut weights_by_address = Vec::with_capacity(worker_set.signers.len());
    for signer in worker_set.signers.into_values() {
        let (Ecdsa(pubkey) | Ed25519(pubkey)) = signer.pub_key;
        weights_by_address.push((pubkey, signer.weight))
    }
    Operators::new(
        weights_by_address,
        worker_set.threshold,
        worker_set.created_at,
    )
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
