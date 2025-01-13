use error_stack::Report;

use crate::messages::Messages;
use crate::proof::Proof;
use crate::{AleoValue, Error};

pub struct ExecuteData {
    proof: Proof,
    payload: Messages,
}

impl ExecuteData {
    pub fn new(proof: Proof, payload: Messages) -> ExecuteData {
        ExecuteData { proof, payload }
    }
}

impl AleoValue for ExecuteData {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ proof: {}, payload: {} }}"#,
            self.proof.to_aleo_string()?,
            self.payload.to_aleo_string()?
        );

        Ok(res)
    }
}
