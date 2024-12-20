use std::str::FromStr;

use cosmwasm_std::HexBinary;
use error_stack::{ensure, Report, Result};
use serde::{Deserialize, Serialize};

use crate::{verify_becnh32, Error};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Address(pub String);

impl Address {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }
}

impl FromStr for Address {
    type Err = Report<Error>;

    fn from_str(address: &str) -> Result<Self, Error> {
        const PREFIX: &str = "aleo";

        ensure!(
            address.len() == 63,
            Error::InvalidAleoAddress(format!(
                "Expected address len is 63. Address '{}' is of len {}",
                address,
                address.len()
            ))
        );

        verify_becnh32(address, PREFIX).map_err(|e| Error::InvalidAleoAddress(e.to_string()))?;

        Ok(Self(address.to_string()))
    }
}

impl TryFrom<&HexBinary> for Address {
    type Error = Report<Error>;

    fn try_from(hex: &HexBinary) -> Result<Self, Error> {
        let address = hex.as_slice();
        let hex_address = hex::encode(address);
        Address::from_str(hex_address.as_str())
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
        let r = Address::from_str(addr);
        println!("-->{r:?}");
        assert_err_contains!(
            Address::from_str(addr),
            crate::Error,
            crate::Error::InvalidAleoAddress(..)
        );

        let addr = "aleo2pqgvl3prke38qwyywqhgd0qu44msp3wks4cqpk3d8m8vxu30wvfql7nmvs";
        assert_err_contains!(
            Address::from_str(addr),
            crate::Error,
            crate::Error::InvalidAleoAddress(..)
        );
    }
}
