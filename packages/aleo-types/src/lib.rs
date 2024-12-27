pub mod address;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid aleo address: '{0}'")]
    InvalidAleoAddress(String),
    #[error("Bech32m verification failed")]
    Bech32m(#[from] bech32::primitives::decode::CheckedHrpstringError),
    #[error("Bech32m: {0}")]
    Bech32mLocalVerification(String),
}

use bech32::primitives::decode::CheckedHrpstring;
use bech32::Bech32m;
use error_stack::{bail, ensure, Report};

fn verify_bech32m(input: &str, prefix: &str) -> Result<(), Report<Error>> {
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
