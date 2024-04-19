use std::fmt;
use std::str::FromStr;

use connection_router_api::ChainName;
use starknet_core::types::FieldElement;
use starknet_core::utils::parse_cairo_short_string;

use crate::types::Hash;

/// This is the event emitted by the gateway cairo contract on Starknet,
/// when the call_contract method is called from a third party.
pub struct ContractCalledEvent {
    pub destination_address: String,
    pub destination_chain: String,
    pub source_address: String,
    pub payload_hash: Hash,
}

impl TryFrom<starknet_core::types::Event> for ContractCalledEvent {
    type Error = EventParseError;

    fn try_from(starknet_event: starknet_core::types::Event) -> Result<Self, Self::Error> {
        if starknet_event.keys.len() != 2 {
            return Err(EventParseError {
                message: "ContractCalled should have exactly 2 event keys".to_owned(),
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

        if event_type != EventType::ContractCalled {
            return Err(EventParseError {
                message: "not a ContractCalled event".to_owned(),
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

        // destination_contract_address (Span<u8>) array length is the second data
        // argument It's an array of bytes, because the contract address might
        // overflow the felt's 252 bit length.
        let destination_address_felt = starknet_event.data[1];
        let destination_address_word_count: u32 = match destination_address_felt.try_into() {
            Ok(da) => da,
            Err(err) => {
                return Err(EventParseError {
                    message: format!("failed to parse destination_chain: {}", err),
                })
            }
        };

        // let destination_address_byte_array: Vec<u8> = vec![0u8;
        // destination_address_bytes_length];

        Ok(ContractCalledEvent {
            destination_address: todo!(),
            destination_chain,
            source_address: todo!(),
            payload_hash: todo!(),
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

/// All Axelar event types supported by starknet
#[derive(Eq, PartialEq)]
enum EventType {
    ContractCalled,
}

impl TryFrom<FieldElement> for EventType {
    type Error = &'static str;

    fn try_from(event_type_felt: FieldElement) -> Result<Self, Self::Error> {
        let event_type_str = parse_cairo_short_string(&event_type_felt);
        let event_type_result = match event_type_str {
            Ok(et) => et,
            Err(_) => return Err("failed to convert felt to an event type, due to failed parsing"),
        };

        match event_type_result.as_str() {
            "ContractCalled" => Ok(EventType::ContractCalled),
            _ => return Err("failed to convert felt to an event type, due to unknown event type"),
        }
    }
}
