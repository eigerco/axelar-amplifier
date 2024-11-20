use std::fmt::Display;
use std::str::FromStr;

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{verify_becnh32, Error};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct Transition {
    transition_id: String,
}

impl FromStr for Transition {
    type Err = Report<Error>;

    fn from_str(transition_id: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        const PREFIX: &str = "au";

        verify_becnh32(transition_id, PREFIX)
            .change_context(Error::InvalidAleoTransition(transition_id.to_string()))?;

        Ok(Self {
            transition_id: transition_id.to_string(),
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
