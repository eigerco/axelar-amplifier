pub mod address;
pub mod transaction;
pub mod transition;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid aleo address: {0}")]
    InvalidAddress(String),
    #[error("invalid aleo transition id: {0}")]
    InvalidAleoTransition(String),
    #[error("invalid aleo transaction id: {0}")]
    InvalidAleoTransaction(String),
}

pub use address::*;
pub use transaction::*;
pub use transition::*;