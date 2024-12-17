use std::str::FromStr as _;

use aleo_types::program::Program;
use error_stack::{bail, Report, ResultExt};
use thiserror::Error;
use bitvec::prelude::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid program name: {0}")]
    InvalidProgramName(String),
}

#[derive(Debug, Clone)]
pub struct Hash(pub [u8; 32]);

#[derive(Debug, Clone)]
pub struct Message {
    pub bits: BitVec<u8, Lsb0>,
}

trait AleoEncode {
    fn aleo_encode(&self) -> Vec<u8>;
}

pub struct Proof {}
pub struct WeightedSigners {}

impl TryFrom<&router_api::Message> for Message {
    type Error = Report<Error>;

    fn try_from(value: &router_api::Message) -> Result<Self, Self::Error> {
        // start struct allocation
        let mut bits = BitVec::new();
        struct_variant(&mut bits);

        todo!()
        // Ok(Self {
        //     source_chain: value.cc_id.source_chain.to_string(),
        //     message_id: value.cc_id.message_id.to_string(),
        //     source_address: value.source_address.to_string(),
        //     contract_address: Program::from_str(value.destination_address.as_str())
        //         .map_err(|e| Error::InvalidProgramName(e.to_string()))?,
        //     payload_hash: Hash(value.payload_hash),
        // })
    }
}

fn struct_variant(bits: &mut BitVec<u8, Lsb0>) {
    bits.push(false);
    bits.push(true);
}

pub struct Messages(Vec<Message>);

impl From<Vec<Message>> for Messages {
    fn from(v: Vec<Message>) -> Self {
        Messages(v)
    }
}

impl TryFrom<Messages> for [u8; 32] {
    type Error = Error;

    fn try_from(value: Messages) -> Result<Self, Error> {
        Ok([0u8; 32])
    }
}

impl Messages {
    pub fn messages_approval_hash(&self) -> Result<[u8; 32], Error> {
        Ok([0u8; 32])
    }
}
