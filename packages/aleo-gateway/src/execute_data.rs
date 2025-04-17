use error_stack::Report;
use multisig::verifier_set::VerifierSet;

use crate::proof::Proof;
use crate::{AleoValue, Error, Messages, WeightedSigners};

pub struct ExecuteDataMessages {
    proof: Proof,
    payload: Messages,
}

impl ExecuteDataMessages {
    pub fn new(proof: Proof, payload: Messages) -> ExecuteDataMessages {
        ExecuteDataMessages { proof, payload }
    }
}

impl AleoValue for ExecuteDataMessages {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ proof: {}, message: {} }}"#,
            self.proof.to_aleo_string()?,
            self.payload.to_aleo_string()?
        );

        Ok(res)
    }
}

pub struct ExecuteDataVerifierSet {
    proof: Proof,
    payload: WeightedSigners,
}

impl ExecuteDataVerifierSet {
    pub fn new(proof: Proof, payload: VerifierSet) -> ExecuteDataVerifierSet {
        let payload = WeightedSigners::try_from(&payload).unwrap();
        ExecuteDataVerifierSet { proof, payload }
    }
}

impl AleoValue for ExecuteDataVerifierSet {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ proof: {}, payload: {} }}"#,
            self.proof.to_aleo_string()?,
            self.payload.to_aleo_string()?
        );

        Ok(res)
    }
}
