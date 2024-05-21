use cosmwasm_std::{Addr, Uint256};
use multisig::{key::PublicKey, msg::Signer, worker_set::WorkerSet};
use router_api::{CrossChainId, Message, CHAIN_NAME_DELIMITER};

use crate::payload::Payload;

pub(super) trait Visitor {
    fn visit_payload(&mut self, payload: &Payload) {
        match payload {
            Payload::Messages(messages) => {
                self.tag(b"message");
                for message in messages {
                    self.visit_message(message);
                }
            }
            Payload::WorkerSet(worker_set) => {
                self.tag(b"worker_set");
                self.visit_worker_set(worker_set)
            }
        }
    }

    fn visit_message(&mut self, message: &Message) {
        self.visit_cc_id(&message.cc_id);
        self.visit_string(message.source_address.as_str());
        self.visit_string(message.destination_chain.as_ref());
        self.visit_string(message.destination_address.as_str());
        self.visit_bytes(&message.payload_hash);
    }

    /// Visit Message's CCID following its `Display` implementation.
    fn visit_cc_id(&mut self, cc_id: &CrossChainId) {
        let mut delimiter_buffer = [0u8; 4];
        let chain_delimiter = CHAIN_NAME_DELIMITER.encode_utf8(&mut delimiter_buffer);

        self.visit_string(cc_id.chain.as_ref());
        self.visit_bytes(chain_delimiter.as_bytes());
        self.visit_string(&cc_id.id);
    }

    fn visit_worker_set(&mut self, worker_set: &WorkerSet) {
        for signer in worker_set.signers.values() {
            self.visit_signer(signer);
        }
        self.visit_u256(&worker_set.threshold);
        self.visit_u64(&worker_set.created_at)
    }

    fn visit_public_key(&mut self, public_key: &PublicKey) {
        match public_key {
            PublicKey::Ecdsa(bytes) => {
                self.tag(b"ecdsa");
                self.visit_bytes(bytes.as_slice())
            }
            PublicKey::Ed25519(bytes) => {
                self.tag(b"ed25519");
                self.visit_bytes(bytes.as_slice())
            }
        }
    }

    fn visit_address(&mut self, address: &Addr) {
        self.visit_bytes(address.as_bytes())
    }

    fn visit_signer(&mut self, signer: &Signer) {
        self.visit_address(&signer.address);
        self.visit_u256(&signer.weight);
        self.visit_public_key(&signer.pub_key)
    }

    fn visit_string(&mut self, string: &str) {
        self.visit_bytes(string.as_bytes())
    }

    fn visit_u64(&mut self, number: &u64);
    fn visit_u256(&mut self, number: &Uint256);
    fn visit_bytes(&mut self, bytes: &[u8]);

    /// No-op by default.
    fn tag(&mut self, _bytes: &[u8]) {}
}
