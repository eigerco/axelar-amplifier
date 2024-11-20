use std::fmt::Display;
use std::str::FromStr;

use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use error_stack::Report;

use super::Error;

pub struct AleoMessageId {
    pub transaction_id: Transaction,
    pub transition_id: Transition,
}

impl AleoMessageId {
    pub fn new(transaction_id: Transaction, transition_id: Transition) -> Self {
        Self {
            transaction_id,
            transition_id,
        }
    }
}

impl FromStr for AleoMessageId {
    type Err = Report<Error>;

    fn from_str(message_id: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        let mut parts = message_id.split("-");

        let transaction_id = Transaction::from_str(
            parts
                .next()
                .ok_or(Error::InvalidAleoMessageIdFormat(message_id.to_owned()))?,
        )
        .map_err(|e| e.change_context(Error::InvalidAleoMessageId(message_id.to_string())))?;

        let transition_id = Transition::from_str(
            parts
                .next()
                .ok_or(Error::InvalidAleoMessageIdFormat(message_id.to_owned()))?,
        )
        .map_err(|e| e.change_context(Error::InvalidAleoMessageId(message_id.to_string())))?;

        Ok(Self {
            transaction_id,
            transition_id,
        })
    }
}

impl Display for AleoMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.transaction_id, self.transition_id)
    }
}
