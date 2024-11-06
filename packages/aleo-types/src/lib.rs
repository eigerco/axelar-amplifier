pub mod address;
pub mod transition;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid address length: {:?}", .0)]
    InvalidAddressLength(Vec<u8>),
    #[error("invalid address: {0}")]
    InvalidAddress(String),
    #[error("invalid aleo transition id")]
    InvalidAleoTransition(String),
}

pub use address::*;
pub use transition::*;
