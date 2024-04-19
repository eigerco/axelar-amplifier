use axelar_wasm_std::voting::Vote;
use mockall::automock;
use starknet_core::types::StarknetError;
use thiserror::Error;
use tonic::async_trait;

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
    async fn verify(&self, tx_hash: &str) -> core::result::Result<Vote, VerifierError>;
}

pub struct RPCMessageVerifier {
    verifier: StarknetClient,
}

impl RPCMessageVerifier {
    pub fn new(url: impl AsRef<str>) -> Self {
        Self {
            verifier: StarknetClient::new(url).unwrap(), /* todoo scale error ? */
        }
    }
}

#[async_trait]
impl MessageVerifier for RPCMessageVerifier {
    /// Verify that a tx with a certain `tx_hash` has happened on Starknet.
    /// `tx_hash` comes from the the Axelar `Message::tx_id`
    async fn verify(&self, tx_hash: &str) -> core::result::Result<Vote, VerifierError> {
        let event = self
            .verifier
            .get_event_by_hash(tx_hash)
            .await
            .map_err(VerifierError::FetchEvent)?;

        // unimplemented!("Check that `Message` model carries necesary
        // inoformation for finding event in starknet side.")
        Ok(Vote::SucceededOnChain)
    }
}
