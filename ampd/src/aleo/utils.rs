use aleo_utils_temp::block_processor::IdValuePair;
use error_stack::Result;
use error_stack::ResultExt;
use snarkvm::prelude::Field;
use snarkvm::prelude::Group;
use snarkvm::prelude::Literal;
use snarkvm::prelude::LiteralType;
use snarkvm::prelude::Network;

use crate::aleo::error::Error;
use crate::aleo::receipt_builder::CallContract;

pub fn read_call_contract(outputs: &IdValuePair) -> Result<CallContract, Error> {
    let value = outputs.value.as_ref().ok_or(Error::CallContractNotFound)?;

    serde_aleo::from_str::<CallContract>(value).change_context(Error::CallContractNotFound)
}

pub fn find_call_contract_in_outputs<N: Network>(
    outputs: &[IdValuePair],
    payload_hash: Field<N>,
) -> Option<String> {
    outputs.iter().find_map(|output| {
        let output_hash = output.value.as_ref().and_then(|value| {
            let group: Group<N> = aleo_gateway::aleo_hash(value).ok()?;
            let literal = Literal::Group(group);
            let literal = literal.cast_lossy(LiteralType::Field).ok()?;
            let Literal::Field(field) = literal else {
                return None;
            };
            Some(field)
        });
        if let Some(output_hash) = output_hash {
            if output_hash == payload_hash {
                output.value.clone()
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Generic function to find a specific type in the outputs
pub fn find_in_outputs<T: for<'de> serde::Deserialize<'de>>(outputs: &[IdValuePair]) -> Option<T> {
    // Only proceed if there's exactly one output
    if outputs.len() != 1 {
        return None;
    }

    let value = &outputs.first()?.value;
    let json = aleo_utils_temp::json_like::into_json(value.as_ref()?).ok()?;
    serde_json::from_str(&json).ok()
}
