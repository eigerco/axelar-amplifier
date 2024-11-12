use std::fmt::{self, Display};
use std::str::FromStr;

use bech32::primitives::decode::CheckedHrpstring;
use bech32::Bech32m;
use error_stack::{bail, Report, ResultExt};
use lazy_static::lazy_static;
use regex::Regex;

use super::Error;

pub struct Bech32mFormat {
    pub encoded: String,
}

impl Bech32mFormat {
    pub fn new(encoded: String) -> Self {
        Self { encoded }
    }
}

const PATTERN: &str = "^([0-9ac-hj-np-z]{8,90})$";

lazy_static! {
    static ref REGEX: Regex = Regex::new(PATTERN).expect("invalid regex");
}

impl FromStr for Bech32mFormat {
    type Err = Report<Error>;

    fn from_str(message_id: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        let (_, [string]) = REGEX
            .captures(message_id)
            .ok_or(Error::InvalidMessageID {
                id: message_id.to_string(),
                expected_format: "Bech32m".to_string(),
            })?
            .extract();

        verify_bech32m(string)?;

        Ok(Self {
            encoded: string.to_string(),
        })
    }
}

impl Display for Bech32mFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encoded)
    }
}

fn verify_bech32m(input: &str) -> Result<(), Report<Error>> {
    let checked = CheckedHrpstring::new::<Bech32m>(input)
        .change_context(Error::InvalidBech32m(input.to_string()))?;

    if checked.data_part_ascii_no_checksum().is_empty() {
        bail!(Error::InvalidBech32m(format!(
            "Message Id is missing the data part: '{input}'"
        )));
    }

    Ok(())
}
