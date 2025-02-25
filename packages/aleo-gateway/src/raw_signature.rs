use error_stack::Report;

use crate::{AleoValue, Error};

// TODO: nonce is skipped
#[derive(Clone, Debug)]
pub struct RawSignature {
    pub signature: Vec<u8>,
}

impl AleoValue for RawSignature {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = String::from_utf8(self.signature.clone()).map_err(|e| {
            Report::new(Error::AleoGateway(format!(
                "Failed to convert to utf8: {}",
                e
            )))
        })?;
        Ok(res)
    }
}
