use std::str::FromStr;

use error_stack::{Report, ResultExt};
use snarkvm_wasm::console::network::MainnetV0;
use snarkvm_wasm::console::program::Network;

use super::Error;

pub struct AleoTransition {
    transition_id: <MainnetV0 as Network>::TransitionID,
}

impl AleoTransition {
    pub fn transition_id(&self) -> &<MainnetV0 as Network>::TransitionID {
        &self.transition_id
    }
}

impl FromStr for AleoTransition {
    type Err = Report<Error>;

    fn from_str(message_id: &str) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        Ok(Self {
            transition_id: <MainnetV0 as Network>::TransitionID::from_str(message_id).unwrap(),
        })
    }
}
