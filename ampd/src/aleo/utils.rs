use aleo_utils::block_processor::IdValuePair;
use error_stack::ResultExt;
use error_stack::Result;

use crate::aleo::error::Error;
use crate::aleo::receipt_builder::CallContract;

pub fn find_call_contract(outputs: &[IdValuePair]) -> Option<CallContract> {
    // Only proceed if there's exactly one output
    if outputs.len() != 1 {
        return None;
    }

    outputs
        .first()?
        .value
        .as_ref()
        .and_then(|value| serde_aleo::from_str::<CallContract>(value).ok())
    // TODO: is it ok to hide the error here?
}

pub fn read_call_contract(outputs: &IdValuePair) -> Result<CallContract, Error> {
    let value = outputs
        .value
        .as_ref()
        .ok_or(Error::CallContractNotFound)?;

    serde_aleo::from_str::<CallContract>(value)
        .change_context(Error::CallContractNotFound)
}

pub fn find_call_contract_in_outputs(
    outputs: &[IdValuePair],
    target_call_contract: &CallContract,
) -> Option<usize> {
    outputs.iter().position(|output| {
        read_call_contract(output).map_or(false, |call_contract| {
            call_contract == *target_call_contract
        })
    })
}

/// Generic function to find a specific type in the outputs
pub fn find_in_outputs<T: for<'de> serde::Deserialize<'de>>(outputs: &[IdValuePair]) -> Option<T> {
    // Only proceed if there's exactly one output
    if outputs.len() != 1 {
        return None;
    }

    let value = &outputs.first()?.value;
    let json = aleo_utils::json_like::into_json(value.as_ref()?).ok()?;
    serde_json::from_str(&json).ok()
}
