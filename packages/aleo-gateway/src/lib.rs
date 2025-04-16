use std::str::FromStr as _;

use error_stack::Report;
use snarkvm_cosmwasm::network::Network;
use snarkvm_cosmwasm::program::ToBits;
use snarkvm_cosmwasm::types::Group;
use thiserror::Error;

mod execute_data;
mod message;
mod message_group;
mod messages;
mod payload_digest;
mod proof;
mod raw_signature;
mod signer_with_signature;
mod weighted_signer;
mod weighted_signers;

pub use execute_data::*;
pub use message::*;
pub use message_group::*;
pub use messages::*;
pub use payload_digest::*;
pub use proof::*;
pub use raw_signature::*;
pub use signer_with_signature::*;
pub use weighted_signer::*;
pub use weighted_signers::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("AleoGateway: {0}")]
    AleoGateway(String),
    #[error("Unsupported Public Key: {0}")]
    UnsupportedPublicKey(String),
    #[error("Aleo: {0}")]
    Aleo(#[from] snarkvm_cosmwasm::program::Error),
    #[error("Hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("AleoTypes: {0}")]
    AleoTypes(#[from] aleo_types::Error),
    #[error("InvalidSourceChainLength: expected: {expected}, actual: {actual}")]
    InvalidEncodedStringLength { expected: usize, actual: usize },
    #[error("Invalid ascii character")]
    InvalidAscii,
    #[error("StringEncoder: {0}")]
    StringEncoder(#[from] aleo_utils::string_encoder::Error),
    #[error("InvalidMessageGroupLength: expected: {max}, actual: {actual}")]
    InvalidMessageGroupLength { max: usize, actual: usize },
    #[error("The number of address signatures ({address_signatures}) does not match the number of signer signatures ({signer_signatures}).")]
    MismatchedSignerCount {
        address_signatures: usize,
        signer_signatures: usize,
    },
    #[error("Checked division failed: {0} / {1}")]
    CheckedDivision(usize, usize),
    #[error("Checked remainder failed: {0} % {1}")]
    CheckedRemainder(usize, usize),
}

pub trait AleoValue {
    fn to_aleo_string(&self) -> Result<String, Report<Error>>;

    fn hash<N: Network>(&self) -> Result<[u8; 32], Report<Error>> {
        let input = self.to_aleo_string()?;
        hash::<std::string::String, N>(input)
    }

    fn bhp<N: Network>(&self) -> Result<Group<N>, Report<Error>> {
        let input = self.to_aleo_string()?;
        aleo_hash::<std::string::String, N>(input)
    }

    fn bhp_string<N: Network>(&self) -> Result<String, Report<Error>> {
        let input = self.to_aleo_string()?;
        aleo_hash::<std::string::String, N>(input).map(|g| g.to_string())
    }
}

pub fn aleo_hash<T: AsRef<str>, N: Network>(input: T) -> Result<Group<N>, Report<Error>> {
    let aleo_value: Vec<bool> = snarkvm_cosmwasm::program::Value::<N>::from_str(input.as_ref())
        .map_err(|e| {
            Report::new(Error::Aleo(e))
                .attach_printable(format!("input: '{:?}'", input.as_ref().to_owned()))
        })?
        .to_bits_le();

    let group = N::hash_to_group_bhp256(&aleo_value).map_err(|e| {
        Report::new(Error::Aleo(e)).attach_printable(format!(
            "Failed to get bhp256 hash: '{:?}'",
            input.as_ref().to_owned()
        ))
    })?;

    Ok(group)
}

pub fn hash<T: AsRef<str>, N: Network>(input: T) -> Result<[u8; 32], Report<Error>> {
    let aleo_value: Vec<bool> = snarkvm_cosmwasm::program::Value::<N>::from_str(input.as_ref())
        .map_err(|e| {
            Report::new(Error::Aleo(e))
                .attach_printable(format!("input: '{:?}'", input.as_ref().to_owned()))
        })?
        .to_bits_le();

    let bits = N::hash_keccak256(&aleo_value).map_err(|e| {
        Report::new(Error::Aleo(e))
            .attach_printable(format!("input2: '{:?}'", input.as_ref().to_owned()))
    })?;

    let mut hash = [0u8; 32];
    for (i, b) in bits.chunks(8).enumerate() {
        let mut byte = 0u8;
        for (i, bit) in b.iter().enumerate() {
            if *bit {
                byte |= 1 << i;
            }
        }
        hash[i] = byte;
    }

    Ok(hash)
}
