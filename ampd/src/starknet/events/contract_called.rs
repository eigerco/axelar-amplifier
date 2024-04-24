use std::fmt;

use ethers::types::H256;
use starknet_core::utils::parse_cairo_short_string;

use crate::starknet::events::EventType;
use crate::starknet::types::byte_array::ByteArray;
use crate::types::Hash;

/// This is the event emitted by the gateway cairo contract on Starknet,
/// when the call_contract method is called from a third party.
#[derive(Debug)]
pub struct ContractCallEvent {
    pub destination_address: String,
    pub destination_chain: String,
    pub source_address: String,
    pub payload_hash: Hash,
}

impl TryFrom<starknet_core::types::Event> for ContractCallEvent {
    type Error = EventParseError;

    fn try_from(starknet_event: starknet_core::types::Event) -> Result<Self, Self::Error> {
        if starknet_event.keys.len() != 2 {
            return Err(EventParseError {
                message: "ContractCall should have exactly 2 event keys".to_owned(),
            });
        }

        // first key is always the event type
        let event_type_felt = starknet_event.keys[0];
        let event_type_result = EventType::try_from(event_type_felt);
        let event_type = match event_type_result {
            Ok(et) => et,
            Err(err) => {
                return Err(EventParseError {
                    message: err.to_owned(),
                });
            }
        };

        if event_type != EventType::ContractCall {
            return Err(EventParseError {
                message: "not a ContractCall event".to_owned(),
            });
        }

        // destination_chain is the second key in the event keys list (the first key
        // defined from the event)
        //
        // This field, should not exceed 252 bits (a felt's length)
        let destination_chain = match parse_cairo_short_string(&starknet_event.keys[1]) {
            Ok(dc) => dc,
            Err(err) => {
                return Err(EventParseError {
                    message: format!("failed to parse destination_chain: {}", err),
                })
            }
        };

        // source_address represents the original callContract sender and
        // is the first field in data, by the order defined in the event.
        let source_address = match parse_cairo_short_string(&starknet_event.data[0]) {
            Ok(sa) => sa,
            Err(err) => {
                return Err(EventParseError {
                    message: format!("failed to parse source_address: {}", err),
                })
            }
        };

        // destination_contract_address (ByteArray) is composed of FieldElements
        // from the second element to elemet X.
        let destination_address_chunks_count_felt = starknet_event.data[1];
        let destination_address_chunks_count: u32 =
            match destination_address_chunks_count_felt.try_into() {
                Ok(da) => da,
                Err(err) => {
                    return Err(EventParseError {
                        message: format!("failed to parse destination_address: {}", err),
                    })
                }
            };

        let da_chunks_count_usize = match usize::try_from(destination_address_chunks_count) {
            Ok(da) => da,
            Err(err) => {
                return Err(EventParseError {
                    message: format!("failed to parse chunks count: {}", err),
                })
            }
        };

        // It's + 3, because we need to offset the 0th element, pending_word and
        // pending_word_count, in addition to all chunks (da_chunks_count_usize)
        let da_elements_start_index: usize = 1;
        let da_elements_end_index: usize = da_chunks_count_usize + 3;
        let destination_address_byte_array: ByteArray = match ByteArray::try_from(
            starknet_event.data[da_elements_start_index..=da_elements_end_index].to_vec(),
        ) {
            Ok(ba) => ba,
            Err(err) => {
                return Err(EventParseError {
                    message: format!("failed to get byte_array chunks: {}", err),
                })
            }
        };

        let destination_address = match destination_address_byte_array.try_to_string() {
            Ok(da) => da,
            Err(err) => {
                return Err(EventParseError {
                    message: format!("failed to convert byte_array to string: {}", err),
                })
            }
        };

        // payload_hash is a keccak256, which is a combination of two felts (chunks)
        // - first felt contains the 128 least significat bits (LSB)
        // - second felt contains the 128 most significat bits (MSG)
        let ph_chunk1_index: usize = da_elements_end_index + 1;
        let ph_chunk2_index: usize = ph_chunk1_index + 1;
        let mut payload_hash = [0; 32];
        let lsb: [u8; 32] = starknet_event.data[ph_chunk1_index].to_bytes_be();
        let msb: [u8; 32] = starknet_event.data[ph_chunk2_index].to_bytes_be();

        // most significat bits, go before least significant bits for u256 construction
        // check - https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_in_u256_values
        payload_hash[..16].copy_from_slice(&msb[16..]);
        payload_hash[16..].copy_from_slice(&lsb[16..]);

        Ok(ContractCallEvent {
            destination_address,
            destination_chain,
            source_address,
            payload_hash: H256::from_slice(&payload_hash),
        })
    }
}

/// An error, representing failure to convert/parse a starknet event
/// to some specific event.
#[derive(Debug, Clone)]
pub struct EventParseError {
    message: String,
}

impl fmt::Display for EventParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to parse starknet event: {}", self.message)
    }
}
