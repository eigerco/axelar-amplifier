use std::fmt::Display;
use std::str::FromStr;

use axelar_wasm_std::nonempty;
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use valuable::Valuable;

use crate::{verify_becnh32, Error};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, Valuable)]
pub struct Transition(nonempty::String);

impl FromStr for Transition {
    type Err = Report<Error>;

    fn from_str(transition_id: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        const PREFIX: &str = "au";

        verify_becnh32(transition_id, PREFIX)
            .change_context(Error::InvalidAleoTransition(transition_id.to_string()))?;

        Ok(Self(transition_id.try_into().map_err(
            |e: axelar_wasm_std::nonempty::Error| Error::InvalidAleoTransition(e.to_string()),
        )?))
    }
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Transition {
    fn as_ref(&self) -> &str {
        &self.0
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
