use aleo_utils::block_processor::IdValuePair;
use aleo_utils::json_like;
use error_stack::{Report, Result};
use tracing::debug;

use crate::aleo::error::Error;
use crate::aleo::receipt_builder::CallContract;

#[derive(Default, Debug)]
pub struct ParsedOutput {
    pub payload: Vec<u8>,
    pub call_contract: CallContract,
}

/// Find the call contract from outputs.
/// CallContract consist of the CallContract data and the raw payload.
pub fn parse_user_output(outputs: &[IdValuePair]) -> Result<ParsedOutput, Error> {
    // When there are only 2 outputs, the first is the CallContract and the second is the raw payload.
    // When there are 3 outputs, the first is the CallContract, the second is the raw payload, and the third is a future.
    // TODO: enforce the above comments
    if !(outputs.len() == 2 || outputs.len() == 3) {
        return Err(Report::new(Error::UserCallnotFound)
            .attach_printable(format!("Expected exactly 2 outputs, got {}", outputs.len())));
    }

    let mut parsed_output = ParsedOutput::default();

    // TODO: handle: we are using take 2 here because if the third output is a future, we fail to parse it.
    for output in outputs.iter().take(2) {
        if let Some(plaintext) = &output.value {
            // Convert to JSON with proper error handling
            let json = json_like::into_json(plaintext.as_str()).map_err(|_| {
                Error::JsonParse(format!("Failed to convert output to JSON: {}", plaintext))
            })?;

            // Try to parse as CallContract
            match serde_json::from_str::<CallContract>(&json) {
                Ok(call_contract) => {
                    parsed_output.call_contract = call_contract;
                }
                Err(e) => {
                    debug!("Failed to parse as CallContract: {}", e);

                    // Store it as the raw payload by directly converting bytes
                    parsed_output.payload = plaintext.as_bytes().to_vec();
                }
            }
        }
    }

    // Validate that we parsed something
    if parsed_output.call_contract == CallContract::default() || parsed_output.payload.is_empty() {
        return Err(Report::new(Error::UserCallnotFound)
            .attach_printable("No valid user output found in transaction"));
    }

    Ok(parsed_output)
}

/// Generic function to find a specific type in the outputs
pub fn find_in_outputs<T: for<'de> serde::Deserialize<'de>>(outputs: &[IdValuePair]) -> Option<T> {
    // Only proceed if there's exactly one output
    if outputs.len() != 1 {
        return None;
    }

    let value = &outputs.first()?.value;
    let json = json_like::into_json(value.as_ref()?).ok()?;
    serde_json::from_str(&json).ok()
}
