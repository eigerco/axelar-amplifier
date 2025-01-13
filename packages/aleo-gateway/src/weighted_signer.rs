use aleo_types::address::Address;
use error_stack::Report;

use crate::{AleoValue, Error};

#[derive(Debug, Clone)]
pub struct WeightedSigner {
    pub signer: Address,
    pub weight: u128,
}

impl AleoValue for WeightedSigner {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{signer: {}, weight: {}u128}}"#,
            self.signer.0, self.weight
        );

        Ok(res)
    }
}
