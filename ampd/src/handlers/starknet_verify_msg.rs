use async_trait::async_trait;
use axelar_wasm_std::voting::{PollId, Vote};
use serde::Deserialize;
use tokio::sync::watch::Receiver;

use crate::event_processor::EventHandler;
use crate::handlers::errors::Error;
use crate::queue::queued_broadcaster::BroadcasterClient;
use crate::starknet::verifier::MessageVerifier;
use crate::types::{Hash, TMAddress};

type Result<T> = error_stack::Result<T, Error>;

#[derive(Deserialize, Debug)]
pub struct Message {
    pub tx_id: String,
    pub event_index: u64,
    pub destination_address: String,
    pub destination_chain: String,
    pub source_address: String,
    pub payload_hash: Hash,
}

pub struct Handler<V, B>
where
    V: MessageVerifier,
    B: BroadcasterClient,
{
    worker: TMAddress,
    voting_verifier: TMAddress,
    msg_verifier: V,
    broadcast_client: B,
    latest_block_height: Receiver<u64>,
}

impl<V, B> Handler<V, B>
where
    V: MessageVerifier + Send + Sync,
    B: BroadcasterClient,
{
    pub fn new(
        worker: TMAddress,
        voting_verifier: TMAddress,
        msg_verifier: V,
        broadcast_client: B,
        latest_block_height: Receiver<u64>,
    ) -> Self {
        Self {
            worker,
            voting_verifier,
            msg_verifier,
            broadcast_client,
            latest_block_height,
        }
    }

    async fn broadcast_votes(&self, poll_id: PollId, votes: Vec<Vote>) -> Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl<V, B> EventHandler for Handler<V, B>
where
    V: MessageVerifier + Send + Sync,
    B: BroadcasterClient + Send + Sync,
{
    type Err = Error;

    async fn handle(&self, event: &events::Event) -> Result<()> {
        unimplemented!()
    }
}
