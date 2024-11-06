use aleo_types::{Address, AleoTransition};
use router_api::ChainName;

use crate::types::Hash;

pub struct Message {
    pub message_id: AleoTransition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address,
    pub payload_hash: Hash,
}
