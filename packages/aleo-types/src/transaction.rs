use std::fmt::Display;
use std::str::FromStr;

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{verify_becnh32, Error};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct Transaction {
    transition_id: String,
}

impl FromStr for Transaction {
    type Err = Report<Error>;

    fn from_str(transaction_id: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        const PREFIX: &str = "at";

        verify_becnh32(transaction_id, PREFIX)
            .change_context(Error::InvalidAleoTransaction(transaction_id.to_string()))?;

        Ok(Self {
            transition_id: transaction_id.to_string(),
        })
    }
}

impl Display for Transaction {
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
    fn validate_aleo_transaction() {
        let addr = "at1pvhkv0gt5qfnljte2nlav7u9rqrafgf04hzwkfu97sctynwghvqskfua6g";
        assert_ok!(Transaction::from_str(addr));
    }

    #[test]
    fn validate_aleo_transaction_errors() {
        let addr = "au1pvhkv0gt5qfnljte2nlav7u9rqrafgf04hzwkfu97sctynwghvqskfua6g";
        assert_err_contains!(
            Transaction::from_str(addr),
            crate::Error,
            crate::Error::InvalidAleoTransaction(..)
        );

        let addr = "at1fnywazjhsvpvga7yszfhye3ftnsd6q35qpmuw4ugl9sghqtmucxqk4ksv9";
        assert_err_contains!(
            Transaction::from_str(addr),
            crate::Error,
            crate::Error::InvalidAleoTransaction(..)
        );
    }
}
