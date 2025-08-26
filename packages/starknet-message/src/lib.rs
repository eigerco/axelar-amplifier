use std::str::FromStr;

use error_stack::{Report, ResultExt};
use router_api::Message as RouterMessage;
use starknet_checked_felt::CheckedFelt;
use starknet_types_core::felt::Felt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid starknet address")]
    InvalidAddress,
}

/// A message that is encoded in the prover and later sent to the Starknet gateway.
#[derive(Clone, Debug, PartialEq)]
pub struct StarknetMessage {
    pub source_chain: String,
    pub message_id: String,
    pub source_address: String,
    pub contract_address: CheckedFelt,
    pub payload_hash: [u8; 32],
}

impl TryFrom<&RouterMessage> for StarknetMessage {
    type Error = Report<Error>;

    fn try_from(msg: &RouterMessage) -> Result<Self, Self::Error> {
        let contract_address = CheckedFelt::from_str(msg.destination_address.as_str())
            .change_context(Error::InvalidAddress)?;

        Ok(StarknetMessage {
            source_chain: msg.cc_id.source_chain.to_string(),
            message_id: msg.cc_id.message_id.to_string(),
            source_address: msg.source_address.to_string(),
            contract_address,
            payload_hash: msg.payload_hash,
        })
    }
}



/// Cairo serialization support for StarknetMessage
pub trait CairoSerialize {
    fn cairo_serialize(&self) -> Vec<Felt>;
}

/// Helper function to serialize a byte slice as Cairo ByteArray
fn serialize_byte_array(bytes: &[u8]) -> Vec<Felt> {
    let mut felts = Vec::new();
    
    // Split into 31-byte chunks for the data array
    let full_chunks: Vec<&[u8]> = bytes.chunks_exact(31).by_ref().collect();
    let remainder = bytes.chunks_exact(31).remainder();
    
    // 1. data array length
    felts.push(Felt::from(full_chunks.len() as u32));
    
    // 2. data array elements (each chunk is exactly 31 bytes)
    for chunk in full_chunks {
        let mut padded = [0u8; 32];
        padded[1..32].copy_from_slice(chunk); // Put 31 bytes starting at index 1
        felts.push(Felt::from_bytes_be(&padded));
    }
    
    // 3. pending_word (the remaining bytes, at most 30)
    if !remainder.is_empty() {
        let mut padded = [0u8; 32];
        padded[32 - remainder.len()..].copy_from_slice(remainder);
        felts.push(Felt::from_bytes_be(&padded));
    } else {
        felts.push(Felt::ZERO);
    }
    
    // 4. pending_word_len
    felts.push(Felt::from(remainder.len() as u32));
    
    felts
}

impl CairoSerialize for StarknetMessage {
    fn cairo_serialize(&self) -> Vec<Felt> {
        let mut felts = Vec::new();
        
        // Serialize source_chain as ByteArray
        felts.extend(serialize_byte_array(self.source_chain.as_bytes()));
        
        // Serialize message_id as ByteArray
        felts.extend(serialize_byte_array(self.message_id.as_bytes()));
        
        // Serialize source_address as ByteArray
        felts.extend(serialize_byte_array(self.source_address.as_bytes()));
        
        // Serialize contract_address as felt
        felts.push(Felt::from_bytes_be(&self.contract_address.to_bytes_be()));
        
        // Serialize payload_hash as U256 (2 felts: low 128-bit, high 128-bit)
        // According to Cairo docs: u256 is serialized as 2 128-bit felts
        let low_bytes = &self.payload_hash[16..32];
        let high_bytes = &self.payload_hash[0..16];
        let low = u128::from_be_bytes(low_bytes.try_into().unwrap());
        let high = u128::from_be_bytes(high_bytes.try_into().unwrap());
        felts.push(Felt::from(low));
        felts.push(Felt::from(high));
        
        felts
    }
}
