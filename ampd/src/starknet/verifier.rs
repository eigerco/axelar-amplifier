use axelar_wasm_std::voting::Vote;
use mockall::automock;
use thiserror::Error;
use tonic::async_trait;

use super::events::contract_call::ContractCallEvent;
use super::json_rpc::{StarknetClient, StarknetClientError};
use crate::handlers::starknet_verify_msg::Message;

#[derive(Error, Debug)]
pub enum VerifierError {
    #[error("JSON-RPC error")]
    JsonRPC,
    #[error("block number missing in JSON-RPC response for finalized block")]
    MissBlockNumber,
    #[error("failed to fetch event: {0}")]
    FetchEvent(#[from] StarknetClientError),
}

#[automock]
#[async_trait]
pub trait MessageVerifier {
    async fn verify_msg(&self, axl_msg: &Message) -> core::result::Result<Vote, VerifierError>;
}

pub struct RPCMessageVerifier {
    client: StarknetClient,
}

impl RPCMessageVerifier {
    pub fn new(url: impl AsRef<str>) -> Self {
        Self {
            client: StarknetClient::new(url).unwrap(), /* todoo scale error ? */
        }
    }
}

#[async_trait]
impl MessageVerifier for RPCMessageVerifier {
    /// Verify that a tx with a certain `tx_hash` has happened on Starknet.
    /// `tx_hash` comes from the the Axelar `Message::tx_id`
    async fn verify_msg(&self, msg: &Message) -> core::result::Result<Vote, VerifierError> {
        match self
            .client
            .get_event_by_hash(msg.tx_id.as_str())
            .await
            .map_err(VerifierError::FetchEvent)?
        {
            Some((event_tx_hash, contract_call_event)) => {
                println!("MESSAGE {:?}", msg);
                println!("CONTRACT_CALL_EVENT {:?}", contract_call_event);
                println!("EVENT_TX_HASH {:?}", event_tx_hash);
                if event_tx_hash == msg.tx_id && contract_call_event == msg
                //     && event.type_ == EventType::ContractCall.struct_tag(gateway_address)
                {
                    Ok(Vote::SucceededOnChain)
                } else {
                    Ok(Vote::FailedOnChain)
                }
            }
            None => Ok(Vote::NotFound),
        }
    }
}

impl PartialEq<&Message> for ContractCallEvent {
    fn eq(&self, axl_msg: &&Message) -> bool {
        axl_msg.source_address == self.source_address
            && axl_msg.destination_chain == self.destination_chain
            && axl_msg.destination_address == self.destination_address
            && axl_msg.payload_hash == self.payload_hash
    }
}
