use aleo_types::{Address, Transition};
use axelar_wasm_std::voting::PollId;
use events_derive::try_from;
use router_api::ChainName;
use serde::{Deserialize, Serialize};

use crate::types::Hash;

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub message_id: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address,
    pub payload_hash: Hash,
}

#[derive(Deserialize, Debug)]
#[try_from("wasm-messages_poll_started")]
struct PollStartedEvent {
    poll_id: PollId,
    source_gateway_address: Address,
    expires_at: u64,
    messages: Vec<Message>,
}
