use axelar_wasm_std::voting::Vote;
use mockall::automock;
use starknet_providers::jsonrpc::HttpTransport;
use thiserror::Error;
use tonic::async_trait;
use url::Url;

use super::events::contract_call::ContractCallEvent;
use super::json_rpc::{Client, StarknetClientError};
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
    client: Client<HttpTransport>,
}

impl RPCMessageVerifier {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::new(HttpTransport::new(Url::parse(url).unwrap())).unwrap(), /* todoo scale error ? */
        }
    }
}

#[async_trait]
impl MessageVerifier for RPCMessageVerifier {
    /// Verify that a tx with a certain `tx_hash` has happened on Starknet.
    /// `tx_hash` comes from the the Axelar `Message::tx_id`
    async fn verify_msg(&self, msg: &Message) -> core::result::Result<Vote, VerifierError> {
        unimplemented!()
    }
}

impl PartialEq<&Message> for ContractCallEvent {
    fn eq(&self, axl_msg: &&Message) -> bool {
        unimplemented!()
    }
}
