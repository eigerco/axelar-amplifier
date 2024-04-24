use starknet_core::types::FieldElement;
use starknet_core::utils::starknet_keccak;

pub mod contract_called;

/// All Axelar event types supported by starknet
#[derive(Eq, PartialEq)]
pub enum EventType {
    ContractCall,
}

// TODO: Can we make this better?
impl TryFrom<FieldElement> for EventType {
    type Error = &'static str;

    fn try_from(event_type_felt: FieldElement) -> Result<Self, Self::Error> {
        let e_type = event_type_felt.to_string();
        let contract_call_type = starknet_keccak("ContractCall".as_bytes()).to_string();

        if e_type == contract_call_type {
            return Ok(EventType::ContractCall);
        } else {
            return Err("failed to convert felt to an event type, due to
        unknown event type");
        }
    }
}
