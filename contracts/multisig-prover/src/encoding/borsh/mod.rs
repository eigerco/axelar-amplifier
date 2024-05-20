use crate::{error::ContractError, payload::Payload};
use axelar_wasm_std::{hash::Hash, operators::Operators};
use multisig::worker_set::WorkerSet;
use router_api::{Message, CHAIN_NAME_DELIMITER};
use sha3::{Digest, Keccak256};

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
    domain_separator: &Hash,
    signer: &WorkerSet,
    payload: &Payload,
) -> Result<Hash, ContractError> {
    let mut hasher = Keccak256::new();
    hasher.update(domain_separator);
    digest_worker_set(&mut hasher, signer);
    digest_payload(&mut hasher, payload);
    Ok(hasher.finalize().into())
}

fn digest_payload(hasher: &mut impl Digest, payload: &Payload) {
    match payload {
        Payload::Messages(messages) => {
            for message in messages {
                digest_message(hasher, message);
            }
        }
        Payload::WorkerSet(worker_set) => digest_worker_set(hasher, worker_set),
    };
}

fn digest_message(hasher: &mut impl Digest, message: &Message) {
    // Hash Message's CCID following its `Display` implementation.
    let mut delimiter_buffer = [0u8; 4];
    let chain_delimiter = CHAIN_NAME_DELIMITER.encode_utf8(&mut delimiter_buffer);
    hasher.update(message.cc_id.chain.as_ref());
    hasher.update(chain_delimiter);
    hasher.update(message.cc_id.id.as_bytes());

    // Hash remaining fields.
    hasher.update(message.source_address.as_str());
    hasher.update(message.destination_chain.as_ref());
    hasher.update(message.destination_address.as_str());
    hasher.update(message.payload_hash);
}

fn digest_worker_set(hasher: &mut impl Digest, worker_set: &WorkerSet) {
    use multisig::key::PublicKey::*;
    // Hash signers.
    for signer in worker_set.signers.values() {
        hasher.update(signer.address.as_bytes());
        hasher.update(signer.weight.to_be_bytes());
        let (Ecdsa(pubkey) | Ed25519(pubkey)) = &signer.pub_key;
        hasher.update(pubkey.as_slice());
    }
    // Hash threshold.
    hasher.update(worker_set.threshold.to_be_bytes());
    // Hash timestamp.
    hasher.update(worker_set.created_at.to_be_bytes());
}
