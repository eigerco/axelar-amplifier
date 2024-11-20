use std::str::FromStr;

use bech32::primitives::decode::CheckedHrpstring;
use bech32::Bech32m;
use error_stack::{ensure, Report, Result, ResultExt};
use serde::{Deserialize, Serialize};

use crate::Error;

const ADDRESS_PREFIX: &str = "aleo";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Address {
    address: String,
}

// impl Address {
//     pub fn as_str(&self) -> &str {
//         self.address.as_str()
//     }
// }

impl FromStr for Address {
    type Err = Report<Error>;

    fn from_str(address: &str) -> Result<Self, Error> {
        let checked = CheckedHrpstring::new::<Bech32m>(address)
            .change_context(Error::InvalidAddress(address.to_string()))?;

        ensure!(
            checked.hrp().as_str() == ADDRESS_PREFIX,
            Error::InvalidAddress(format!("'aleo' address prefix is not matching: {address}"))
        );

        Ok(Self {
            address: address.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use assert_ok::assert_ok;
    use axelar_wasm_std::assert_err_contains;

    use super::*;

    #[test]
    fn validate_aleo_address() {
        let addr = "aleo1pqgvl3prke38qwyywqhgd0qu44msp3wks4cqpk3d8m8vxu30wvfql7nmvs";
        assert_ok!(Address::from_str(addr));
    }

    #[test]
    fn validate_aleo_address_errors() {
        let addr = "aleo1pqgvl3prke38qwyywqhgd0qu44msp3wks4cqpk3d8m8vxu30wvfql7nmv";
        assert_err_contains!(
            Address::from_str(addr),
            crate::Error,
            crate::Error::InvalidAddress(..)
        );

        let addr = "aleo2pqgvl3prke38qwyywqhgd0qu44msp3wks4cqpk3d8m8vxu30wvfql7nmvs";
        assert_err_contains!(
            Address::from_str(addr),
            crate::Error,
            crate::Error::InvalidAddress(..)
        );
    }
}
