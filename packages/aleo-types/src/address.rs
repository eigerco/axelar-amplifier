use std::fmt::Display;
use std::str::FromStr;
use std::sync::LazyLock;

use axelar_wasm_std::nonempty;
use cosmwasm_std::HexBinary;
use error_stack::{ensure, Report, Result};
use serde::{Deserialize, Serialize};

use crate::{verify_becnh32, Error};

pub const ALEO_ADDRESS_LEN: usize = 63;
pub static ZERO_ADDRESS: LazyLock<Address> = LazyLock::new(|| Address::default());

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Hash)]
pub struct Address(pub nonempty::String);

impl Default for Address {
    fn default() -> Self {
        Self(
            nonempty::String::try_from(
                "aleo1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq3ljyzc",
            )
            .unwrap(),
        )
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
            address.len() == ALEO_ADDRESS_LEN,
            Error::InvalidAleoAddress(format!(
                "Expected address len is {}. Address '{}' is of len {}",
                ALEO_ADDRESS_LEN,
                address,
                address.len()
            ))
        );

        verify_becnh32(address, PREFIX).map_err(|e| Error::InvalidAleoAddress(e.to_string()))?;

        Ok(Self(address.try_into().map_err(
            |e: axelar_wasm_std::nonempty::Error| Error::InvalidAleoAddress(e.to_string()),
        )?))
    }
}

impl TryFrom<&HexBinary> for Address {
    type Error = Report<Error>;

    fn try_from(hex: &HexBinary) -> Result<Self, Error> {
        let address =
            std::str::from_utf8(hex).map_err(|e| Error::InvalidAleoAddress(e.to_string()))?;
        Address::from_str(address)
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
