use self::{hasher::PayloadHasher, visitor::Visitor};
use crate::payload::Payload;
use axelar_wasm_std::{hash::Hash, operators::Operators};
use multisig::worker_set::WorkerSet;

pub mod execute_data;
mod hasher;
mod visitor;

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
    domain_separator: &Hash,
    signer: &WorkerSet,
    payload: &Payload,
) -> Hash {
    let mut hasher = PayloadHasher::default();
    hasher.visit_bytes(domain_separator);
    hasher.visit_worker_set(signer);
    hasher.visit_payload(payload);
    hasher.finalize()
}
