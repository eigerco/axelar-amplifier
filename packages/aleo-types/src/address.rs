use std::{fmt::Display, str::FromStr};

use cosmwasm_std::HexBinary;
use error_stack::{ensure, Report, Result};
use serde::{Deserialize, Serialize};

use crate::{verify_becnh32, Error};

pub const ALEO_ADDRESS_LEN: usize = 63;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Address(pub String);

impl Default for Address {
    fn default() -> Self {
        // Self("aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w".to_string())
        Self("aleo1qtrn0h0pakusngjemdehzljqthu3e8vfwl5qj2zccc5cgasutq8qjy2afr".to_string())
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

        Ok(Self(address.to_string()))
    }
}

impl TryFrom<&HexBinary> for Address {
    type Error = Report<Error>;

    fn try_from(hex: &HexBinary) -> Result<Self, Error> {
        let address =
            std::str::from_utf8(&hex).map_err(|e| Error::InvalidAleoAddress(e.to_string()))?;
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
