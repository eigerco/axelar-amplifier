use aleo_types::address::Address;
use aleo_types::transition::Transition;
use aleo_utils::json_like;
use aleo_utils::string_encoder::StringEncoder;
use error_stack::Result;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use tracing::{error, info};

use crate::aleo::error::Error;
use crate::types::Hash;

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CallContract {
    pub(crate) caller: String,
    pub(crate) sender: String,
    pub(crate) destination_chain: Vec<u128>,
    pub(crate) destination_address: Vec<u128>,
}

impl CallContract {
    pub fn destination_chain(&self) -> String {
        let encoded_string = StringEncoder {
            buf: self.destination_chain.clone(),
        };
        encoded_string.decode()
    }

    pub fn destination_address(&self) -> Result<String, error_stack::Report<Error>> {
        let encoded_string = StringEncoder {
            buf: self.destination_address.clone(),
        };
        let ascii_string = encoded_string.decode();
        Ok(ascii_string)
    }
}

#[derive(Debug)]
pub struct CallContractReceipt {
    pub transition: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address,
    pub payload: Vec<u8>,
}

impl PartialEq<crate::handlers::aleo_verify_msg::Message> for CallContractReceipt {
    fn eq(&self, message: &crate::handlers::aleo_verify_msg::Message) -> bool {
        info!(
            "transition_id: chain.{} == msg.{} ({})",
            self.transition,
            message.tx_id,
            self.transition == message.tx_id
        );
        info!(
            "destination_address: chain.{} == msg.{} ({})",
            self.destination_address,
            message.destination_address,
            self.destination_address == message.destination_address
        );
        info!(
            "destination_chain: chain.{} == msg.{} ({})",
            self.destination_chain,
            message.destination_chain,
            self.destination_chain == message.destination_chain
        );
        info!(
            "source_address: chain.{:?} == msg.{:?} ({})",
            self.source_address,
            message.source_address,
            self.source_address == message.source_address
        );

        let payload_hash = match payload_hash(&self.payload, self.destination_chain.as_ref()) {
            Ok(hash) => hash,
            Err(e) => {
                error!("payload_hash: {}", e);
                return false;
            }
        };

        info!(
            "payload_hash: chain.{:?} == msg.{:?} ({})",
            payload_hash,
            message.payload_hash,
            payload_hash == message.payload_hash
        );

        self.transition == message.tx_id
            && self.destination_address == message.destination_address
            && self.destination_chain == message.destination_chain
            && self.source_address == message.source_address
            && payload_hash == message.payload_hash
    }
}

fn payload_hash(payload: &[u8], destination_chain: &str) -> std::result::Result<Hash, Error> {
    let payload = std::str::from_utf8(payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
    let payload_hash = if destination_chain.starts_with("eth") {
        let payload =
            json_like::into_json(payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload: Vec<u8> =
            serde_json::from_str(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload =
            std::str::from_utf8(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload = solabi::encode(&payload);
        let payload_hash = keccak256(&payload).to_vec();
        Hash::from_slice(&payload_hash)
    } else {
        // Keccak + bhp hash
        let payload_hash =
            aleo_gateway::hash::<&str, snarkvm_cosmwasm::network::TestnetV0>(payload)
                .map_err(|e| Error::PayloadHash(e.to_string()))?;
        Hash::from_slice(&payload_hash)
    };

    Ok(payload_hash)
}

fn keccak256(payload: impl AsRef<[u8]>) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(payload);
    hasher.finalize().into()
}
