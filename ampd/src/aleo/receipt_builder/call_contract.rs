use std::marker::PhantomData;

use aleo_string_encoder::StringEncoder;
use aleo_types::transition::Transition;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use snarkvm::prelude::{Address, Field, Network};
use tracing::{error, info};

use crate::aleo::error::Error;
use crate::types::Hash;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "Address<N>: Serialize + for<'a> Deserialize<'a>")]
pub struct CallContract<N: Network> {
    pub(crate) caller: Address<N>,
    pub(crate) sender: Address<N>,
    pub(crate) destination_chain: [u128; 2],
    pub(crate) destination_address: [u128; 6],
    pub(crate) payload_hash: Field<N>,
}

impl<N: Network> CallContract<N> {
    pub fn destination_chain(&self) -> Result<String, Error> {
        Ok(StringEncoder::from_slice(&self.destination_chain).decode()?)
    }

    pub fn destination_address(&self) -> Result<String, Error> {
        Ok(StringEncoder::from_slice(&self.destination_address).decode()?)
    }
}

#[derive(Debug)]
pub struct CallContractReceipt<N: Network> {
    pub transition: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address<N>,
    pub payload: Vec<u8>,
    pub n: PhantomData<N>,
}

impl<N: Network> PartialEq<crate::handlers::aleo_verify_msg::Message<N>>
    for CallContractReceipt<N>
{
    fn eq(&self, message: &crate::handlers::aleo_verify_msg::Message<N>) -> bool {
        info!(
            "transition_id: chain.'{}' == msg.'{}' ({})",
            self.transition,
            message.tx_id,
            self.transition == message.tx_id
        );
        info!(
            "destination_address: chain.'{}' == msg.'{}' ({})",
            self.destination_address,
            message.destination_address,
            self.destination_address == message.destination_address
        );
        info!(
            "destination_chain: chain.'{}' == msg.'{}' ({})",
            self.destination_chain,
            message.destination_chain,
            self.destination_chain == message.destination_chain
        );
        info!(
            "source_address: chain.'{:?}' == msg.'{:?}' ({})",
            self.source_address,
            message.source_address,
            self.source_address == message.source_address
        );

        let payload_hash = match payload_hash::<N>(&self.payload, self.destination_chain.as_ref()) {
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

fn payload_hash<N: Network>(
    payload: &[u8],
    destination_chain: &str,
) -> std::result::Result<Hash, Error> {
    let payload = std::str::from_utf8(payload).map_err(|e| Error::PayloadHash(e.to_string()))?;

    // TODO: We need to figure out how to handle the payload hash for different chains.
    // At least for EVM chains we can group them instead of using the 'eth' prefix at chain name.
    let payload_hash = if destination_chain.starts_with("eth") {
        let payload = aleo_utils_temp::json_like::into_json(payload).map_err(Error::PayloadHash)?;
        let payload: Vec<u8> =
            serde_json::from_str(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload =
            std::str::from_utf8(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload = solabi::encode(&payload);
        let payload_hash = keccak256(&payload).to_vec();
        Hash::from_slice(&payload_hash)
    } else if destination_chain == "axelar" {
        let payload = aleo_program_driver::its::axelarinterchaintokenhub::abi_translate(payload);

        if let Ok(payload) = payload {
            info!(?payload, "Receive axelar payload is an ITS message");
            let payload_hash = keccak256(&payload).to_vec();

            Hash::from_slice(&payload_hash)
        } else {
            todo!("Handle axelar payload hash for non-ITS messages")
        }
    } else {
        // Keccak + bhp hash
        let payload_hash = aleo_gateway::hash::<&str, N>(payload)
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
