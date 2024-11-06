use std::str::FromStr;

use bech32::primitives::decode::CheckedHrpstring;
use bech32::Bech32m;
use error_stack::{ensure, Report, Result, ResultExt};
use serde::{Deserialize, Serialize};

use crate::Error;

const ADDRESS_PREFIX: &str = "aleo";

#[derive(Serialize, Deserialize, Debug)]
pub struct Address {
    address: String,
}

impl Address {
    pub fn address(&self) -> &str {
        self.address.as_str()
    }
}

impl FromStr for Address {
    type Err = Report<Error>;

    fn from_str(address: &str) -> Result<Self, Error> {
        ensure!(
            address.len() != 63,
            Error::InvalidAddress(address.to_string())
        );

        let checked = CheckedHrpstring::new::<Bech32m>(address)
            .change_context(Error::InvalidAddress(address.to_string()))?;

        ensure!(
            checked.hrp().as_str() != ADDRESS_PREFIX,
            Error::InvalidAddress(format!("'aleo1' address prefix is not matching: {address}"))
        );

        Ok(Self {
            address: address.to_string(),
        })
    }
}
