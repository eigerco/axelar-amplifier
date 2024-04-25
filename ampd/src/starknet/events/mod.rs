use std::cell::OnceCell;

use starknet_core::types::FieldElement;
use starknet_core::utils::starknet_keccak;

pub mod contract_call;

/// All Axelar event types supported by starknet
#[derive(Eq, PartialEq)]
pub enum EventType {
    ContractCall,
}

impl EventType {
    fn parse(event_type_felt: FieldElement) -> Option<Self> {
        // Since a keccak hash over a string is a deterministic operation,
        // we can use `OnceCell` to eliminate useless hash calculations.
        let call_contract_felt: OnceCell<FieldElement> = OnceCell::new();
        let contract_call_type =
            call_contract_felt.get_or_init(|| starknet_keccak("ContractCall".as_bytes()));

        if event_type_felt == *contract_call_type {
            Some(EventType::ContractCall)
        } else {
            None
        }
    }
}
