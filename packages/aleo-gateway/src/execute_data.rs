use error_stack::Report;

use crate::proof::Proof;
use crate::{AleoValue, Error, Message};

pub struct ExecuteData {
    proof: Proof,
    payload: Message,
}

impl ExecuteData {
    pub fn new(proof: Proof, payload: Message) -> ExecuteData {
        ExecuteData { proof, payload }
    }
}

impl AleoValue for ExecuteData {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ proof: {}, message: {} }}"#,
            self.proof.to_aleo_string()?,
            self.payload.to_aleo_string()?
        );

        Ok(res)
    }
}
