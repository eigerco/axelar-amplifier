use std::marker::PhantomData;

use aleo_types::address::Address;
use aleo_types::transition::Transition;
use aleo_utils::json_like;
use aleo_utils::string_encoder::StringEncoder;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use snarkvm_cosmwasm::program::Network;
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

    pub fn destination_address(&self) -> String {
        let encoded_string = StringEncoder {
            buf: self.destination_address.clone(),
        };
        encoded_string.decode()
    }
}

#[derive(Debug)]
pub struct CallContractReceipt<N: Network> {
    pub transition: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address,
    pub payload: Vec<u8>,
    pub n: PhantomData<N>,
}

impl<N: Network> PartialEq<crate::handlers::aleo_verify_msg::Message> for CallContractReceipt<N> {
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
        let payload = json_like::into_json(payload).map_err(Error::PayloadHash)?;
        let payload: Vec<u8> =
            serde_json::from_str(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload =
            std::str::from_utf8(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload = solabi::encode(&payload);
        let payload_hash = keccak256(&payload).to_vec();
        Hash::from_slice(&payload_hash)
    } else if destination_chain == "axelar" {
        let payload = its_message_abi(payload)
            .map_err(|_| Error::PayloadHash("Failed to parse ITS message ABI".to_string()))?;
        // TODO: check if the payload is ITS payload

        let payload_hash = keccak256(payload).to_vec();
        println!("---------------------->Payload hash: >{:?}<", payload_hash);

        Hash::from_slice(&payload_hash)
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

use aleo_utils::aleo_json::{AleoJson, RemoteDeployInterchainToken};
// use aleo_utils::serde_plaintext;
use cosmwasm_std::{HexBinary, Uint256};
use interchain_token_service::{HubMessage, TokenId};

fn its_message_abi(payload: &str) -> Result<HexBinary, ()> {
    let r: RemoteDeployInterchainToken = aleo_utils::serde_plaintext::from_str(&payload).unwrap();

    // Convert token id to u256
    // we know that token_id can be stored at u256
    // First we will convert this string to u256
    let bytes = r.token_id[1]
        .to_le_bytes()
        .iter()
        .chain(r.token_id[0].to_le_bytes().iter())
        .cloned()
        .collect::<Vec<u8>>();
    let token_id: Uint256 = Uint256::new(bytes.try_into().unwrap());

    // Then we will store it at [u8; 32]
    let token_id: [u8; 32] = token_id.to_le_bytes();
    // let name = todo!();
    let name = aleo_utils::string_encoder::StringEncoder::from_array(&[r.info.name]);
    let symbol = aleo_utils::string_encoder::StringEncoder::from_array(&[r.info.symbol]);
    let destination_chain =
        aleo_utils::string_encoder::StringEncoder::from_array(&r.destination_chain);

    let msg = interchain_token_service::DeployInterchainToken {
        token_id: TokenId::from(token_id),
        name: axelar_wasm_std::nonempty::String::try_from(name.decode()).unwrap(),
        symbol: axelar_wasm_std::nonempty::String::try_from(symbol.decode()).unwrap(),
        decimals: r.info.decimals,
        minter: None,
    };

    let message_hub = HubMessage::SendToHub {
        destination_chain: router_api::ChainNameRaw::try_from(destination_chain.decode()).unwrap(),
        message: msg.into(),
    };

    println!("--->Parsed RemoteDeployInterchainToken: {:?}", message_hub);
    let bytes = message_hub.abi_encode();
    Ok(bytes)
}
