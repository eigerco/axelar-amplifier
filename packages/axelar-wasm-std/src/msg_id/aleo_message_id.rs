use std::str::FromStr;

use aleo_types::{Transaction, Transition};
use error_stack::Report;

use super::Error;

pub struct AleoMessageId {
    pub transaction_id: Transaction,
    pub transition_id: Transition,
    pub index: u32,
}

impl AleoMessageId {
    pub fn new(transaction_id: Transaction, transition_id: Transition, index: u32) -> Self {
        Self {
            transaction_id,
            transition_id,
            index,
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

        let transition_id = Transition::from_str(
            parts
                .next()
                .ok_or(Error::InvalidAleoMessageIdFormat(message_id.to_owned()))?,
        )
        .map_err(|e| Report::new(Error::InvalidAleoMessageId(e.to_string())))?;

        let transaction_id = Transaction::from_str(
            parts
                .next()
                .ok_or(Error::InvalidAleoMessageIdFormat(message_id.to_owned()))?,
        )
        .map_err(|e| Report::new(Error::InvalidAleoMessageId(e.to_string())))?;

        let index = parts
            .next()
            .ok_or(Error::InvalidAleoMessageIdFormat(message_id.to_owned()))?
            .parse()
            .map_err(|_| Error::EventIndexOverflow(message_id.to_string()))?;

        Ok(Self {
            transition_id,
            transaction_id,
            index,
        })
    }
}
