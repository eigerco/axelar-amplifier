pub mod address;
pub mod transaction;
pub mod transition;

use snarkvm_cosmwasm::network::TestnetV0;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid aleo address: '{0}'")]
    InvalidAleoAddress(String),
    #[error("Invalid aleo transition id: '{0}'")]
    InvalidAleoTransition(String),
    #[error("Invalid aleo transaction id: '{0}'")]
    InvalidAleoTransaction(String),
    #[error("Invalid aleo program name: '{0}'")]
    InvalidProgramName(String),
    #[error("Bech32m verification failed")]
    Bech32m(#[from] bech32::primitives::decode::CheckedHrpstringError),
    #[error("Bech32m: {0}")]
    Bech32mLocalVerification(String),
}

use bech32::primitives::decode::CheckedHrpstring;
use bech32::Bech32m;
use error_stack::{bail, ensure, Report};

fn verify_becnh32(input: &str, prefix: &str) -> Result<(), Report<Error>> {
    let checked = CheckedHrpstring::new::<Bech32m>(input).map_err(Error::Bech32m)?;

    ensure!(
        checked.hrp().as_str() == prefix,
        Error::Bech32mLocalVerification(format!("Failed to validate prefix: '{prefix}'"))
    );

    if checked.data_part_ascii_no_checksum().is_empty() {
        bail!(Error::Bech32mLocalVerification(
            "No data part found".to_string()
        ));
    }

    Ok(())
}

use std::str::FromStr;

use cosmwasm_std::HexBinary;
use snarkvm_cosmwasm::types::Address;

pub fn hexbinary_to_address(hex: &HexBinary) -> Result<Address<TestnetV0>, Report<Error>> {
    let address = std::str::from_utf8(hex).map_err(|e| Error::InvalidAleoAddress(e.to_string()))?;
    Ok(snarkvm_cosmwasm::types::Address::<TestnetV0>::from_str(address).unwrap())
}
