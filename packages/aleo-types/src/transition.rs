use std::str::FromStr;

use bech32::primitives::decode::CheckedHrpstring;
use bech32::Bech32m;
use error_stack::{bail, Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct Transition {
    transition_id: String,
}

impl Transition {
    pub fn transition_id(&self) -> &String {
        &self.transition_id
    }
}

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
