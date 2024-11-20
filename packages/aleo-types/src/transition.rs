use std::fmt::Display;
use std::str::FromStr;

use bech32::primitives::decode::CheckedHrpstring;
use bech32::Bech32m;
use error_stack::{bail, Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct Transition {
    transition_id: String,
}

// impl Transition {
//     pub fn transition_id(&self) -> &str {
//         self.transition_id.as_str()
//     }
// }

impl FromStr for Transition {
    type Err = Report<Error>;

    fn from_str(message_id: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        const PREFIX: &str = "au";

        let checked = CheckedHrpstring::new::<Bech32m>(message_id)
            .change_context(Error::InvalidAleoTransition(message_id.to_owned()))?;

        if checked.hrp().as_str() != PREFIX {
            bail!(Error::InvalidAleoTransition(message_id.to_owned()));
        }

        if checked.data_part_ascii_no_checksum().is_empty() {
            bail!(Error::InvalidAleoTransition(message_id.to_owned()));
        }

        Ok(Self {
            transition_id: message_id.to_string(),
        })
    }
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.transition_id)
    }
}

#[cfg(test)]
mod tests {
    use assert_ok::assert_ok;
    use axelar_wasm_std::assert_err_contains;

    use super::*;

    #[test]
    fn validate_aleo_transition() {
        let addr = "au1fnywazjhsvpvga7yszfhye3ftnsd6q35qpmuw4ugl9sghqtmucxqk4ksv8";
        assert_ok!(Transition::from_str(addr));
    }

    #[test]
    fn validate_aleo_transition_errors() {
        let addr = "at1fnywazjhsvpvga7yszfhye3ftnsd6q35qpmuw4ugl9sghqtmucxqk4ksv8";
        assert_err_contains!(
            Transition::from_str(addr),
            crate::Error,
            crate::Error::InvalidAleoTransition(..)
        );

        let addr = "au1fnywazjhsvpvga7yszfhye3ftnsd6q35qpmuw4ugl9sghqtmucxqk4ksv9";
        assert_err_contains!(
            Transition::from_str(addr),
            crate::Error,
            crate::Error::InvalidAleoTransition(..)
        );
    }
}
