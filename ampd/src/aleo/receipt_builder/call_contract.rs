use std::marker::PhantomData;

use aleo_string_encoder::string_encoder::StringEncoder;
use aleo_types::transition::Transition;
use axelar_wasm_std::nonempty;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use snarkvm::prelude::{Address, Network, TestnetV0};
use tracing::{error, info};

use crate::aleo::error::Error;
use crate::types::Hash;

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CallContract {
    pub(crate) caller: String,
    pub(crate) sender: String,
    pub(crate) destination_chain: [u128; 2],
    pub(crate) destination_address: [u128; 6],
    pub(crate) payload_hash: String, // Field
}

impl CallContract {
    pub fn destination_chain(&self) -> String {
        StringEncoder::from_array(&self.destination_chain).decode()
    }

    pub fn destination_address(&self) -> String {
        StringEncoder::from_array(&self.destination_address).decode()
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
        let payload = aleo_utils::json_like::into_json(payload).map_err(Error::PayloadHash)?;
        let payload: Vec<u8> =
            serde_json::from_str(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload =
            std::str::from_utf8(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload = solabi::encode(&payload);
        let payload_hash = keccak256(&payload).to_vec();
        Hash::from_slice(&payload_hash)
    } else if destination_chain == "axelar" {
        // let payload = its_message_abi(payload)
        let payload = translate(payload);
        // .map_err(|_| Error::PayloadHash("Failed to parse ITS message ABI".to_string()))?;
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

use cosmwasm_std::{HexBinary, Uint256};
use interchain_token_service::{HubMessage, InterchainTransfer, TokenId};

#[derive(Deserialize, Debug, Clone)]
pub struct DeployInterchainToken {
    pub its_token_id: [u128; 2],
    pub name: u128,
    pub symbol: u128,
    pub decimals: u8,
}

// This struct represents the payload of the ITS message
// from Aleo ITS
#[derive(Deserialize, Debug, Clone)]
pub struct RemoteDeployInterchainToken {
    pub info: DeployInterchainToken,
    pub destination_chain: [u128; 2],
    pub has_minter: bool,
    pub minter: [u128; 6],
}

#[derive(Debug, Deserialize)]
pub struct OutgoingInterchainTransfer {
    pub its_token_id: [u128; 2],
    pub source_address: Address<TestnetV0>,
    pub destination_address: [u128; 6],
    pub amount: u128,
}

#[derive(Debug, Deserialize)]
pub struct ItsOutgoingInterchainTransfer {
    pub inner_message: OutgoingInterchainTransfer,
    pub destination_chain: [u128; 2],
}

fn its_outgoing_transfer_abi(transfer: &ItsOutgoingInterchainTransfer) -> HexBinary {
    // Convert token id to u256
    // we know that token_id can be stored at u256
    // First we will convert this string to u256
    let bytes = transfer.inner_message.its_token_id[1]
        .to_le_bytes()
        .iter()
        .chain(transfer.inner_message.its_token_id[0].to_le_bytes().iter())
        .cloned()
        .collect::<Vec<u8>>();
    let token_id: Uint256 = Uint256::new(bytes.try_into().unwrap());

    // Then we will store it at [u8; 32]
    let token_id: [u8; 32] = token_id.to_le_bytes();

    let source_address =
        StringEncoder::encode_string(&transfer.inner_message.source_address.to_string())
            .unwrap()
            .decode();
    let source_address: axelar_wasm_std::nonempty::HexBinary =
        source_address.into_bytes().try_into().unwrap();
    // .map_err(|e| anyhow!("Failed to convert source address: {}", e))?;

    // let destination_address =
    //     StringEncoder::from_array(&transfer.inner_message.destination_address)
    //         .decode()
    //         .into_bytes()
    //         .try_into()
    //         .unwrap();
    // .map_err(|e| anyhow!("Failed to convert destination chain: {}", e))?;

    let d = transfer.inner_message.destination_address;
    let s = format!(
        "{}u128, {}u128, {}u128, {}u128, {}u128, {}u128",
        d[0], d[1], d[2], d[3], d[4], d[5]
    );
    println!("decoded Destination address: '{}'", s);
    let destination_address = StringEncoder::from_aleo_value(&s).unwrap();
    let destination_address = destination_address.decode();
    println!("Destination address as string: {}", destination_address);
    let destination_address = destination_address.strip_prefix("0x").unwrap_or(&destination_address);
    let destination_address = hex::decode(&destination_address).unwrap();

    let msg = InterchainTransfer {
        token_id: TokenId::from(token_id),
        source_address,
        destination_address: axelar_wasm_std::nonempty::HexBinary::try_from(destination_address)
            .unwrap(),
        amount: Uint256::from_u128(transfer.inner_message.amount)
            .try_into()
            .unwrap(),
        data: None,
    };

    let destination_chain = StringEncoder::from_array(&transfer.destination_chain).decode();

    let message_hub = HubMessage::SendToHub {
        destination_chain: router_api::ChainNameRaw::try_from(destination_chain).unwrap(),
        // .map_err(|e| anyhow!("Failed to parse destination chain: {}", e))?,
        message: msg.into(),
    };

    println!("--->Parsed ItsOutgoingInterchainTransfer: {message_hub:?}");

    let bytes = message_hub.abi_encode();
    bytes
}

fn remote_interchain_token_abi(r: &RemoteDeployInterchainToken) -> HexBinary {
    // Convert token id to u256
    // we know that token_id can be stored at u256
    // First we will convert this string to u256
    let bytes = r.info.its_token_id[1]
        .to_le_bytes()
        .iter()
        .chain(r.info.its_token_id[0].to_le_bytes().iter())
        .cloned()
        .collect::<Vec<u8>>();
    let token_id: Uint256 = Uint256::new(bytes.try_into().unwrap());

    // Then we will store it at [u8; 32]
    let token_id: [u8; 32] = token_id.to_le_bytes();
    let name = StringEncoder::from_array(&[r.info.name]);
    let symbol = StringEncoder::from_array(&[r.info.symbol]);
    let destination_chain = StringEncoder::from_array(&r.destination_chain);

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

    println!("--->Parsed RemoteDeployInterchainToken: {message_hub:?}");
    let bytes = message_hub.abi_encode();
    bytes
}

fn translate(message: &str) -> HexBinary {
    let message = if let Ok(outgoing_transfer) = serde_aleo::from_str(message) {
        its_outgoing_transfer_abi(&outgoing_transfer)
    } else if let Ok(remote_deploy_interchain_token) = serde_aleo::from_str(message) {
        remote_interchain_token_abi(&remote_deploy_interchain_token)
    } else {
        panic!("Failed to parse message: {}", message);
    };

    message
}

fn its_message_abi(payload: &str) -> Result<HexBinary, Error> {
    let r: RemoteDeployInterchainToken = serde_aleo::from_str(&payload)?;

    // Convert token id to u256
    // we know that token_id can be stored at u256
    // First we will convert this string to u256
    let bytes = r.info.its_token_id[1]
        .to_le_bytes()
        .iter()
        .chain(r.info.its_token_id[0].to_le_bytes().iter())
        .cloned()
        .collect::<Vec<u8>>();
    let token_id: Uint256 =
        Uint256::new(bytes.try_into().map_err(|_| {
            Error::PayloadHash("Failed to convert token_id to Uint256".to_string())
        })?);

    // Then we will store it at [u8; 32]
    let token_id: [u8; 32] = token_id.to_le_bytes();
    let name = StringEncoder::from_array(&[r.info.name]);
    let symbol = StringEncoder::from_array(&[r.info.symbol]);
    let destination_chain = StringEncoder::from_array(&r.destination_chain);
    let minter = if r.has_minter {
        // convert a [u128] to HexBinary
        let hex = r
            .minter
            .iter()
            .map(|&x| x.to_le_bytes())
            .flatten()
            .collect::<Vec<u8>>();
        Some(nonempty::HexBinary::try_from(hex)?)
    } else {
        None
    };

    let msg = interchain_token_service::DeployInterchainToken {
        token_id: TokenId::from(token_id),
        name: axelar_wasm_std::nonempty::String::try_from(name.decode())?,
        symbol: axelar_wasm_std::nonempty::String::try_from(symbol.decode())?,
        decimals: r.info.decimals,
        minter,
    };

    let message_hub = HubMessage::SendToHub {
        destination_chain: router_api::ChainNameRaw::try_from(destination_chain.decode())?,
        message: msg.into(),
    };

    println!("--->Parsed RemoteDeployInterchainToken: {:?}", message_hub);
    let bytes = message_hub.abi_encode();
    Ok(bytes)
}
