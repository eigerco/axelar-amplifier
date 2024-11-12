//use cosmwasm_std::Uint256;
use std::str::FromStr;

use error_stack::{Report, ResultExt};
use ethers_core::abi::{InvalidOutputType, Token, Tokenizable};
use ethers_core::types::U256;
use router_api::Message as RouterMessage;
use starknet_core::types::FieldElement;

use crate::error::Error;

pub mod error;

#[derive(Clone, Debug)]
pub struct StarknetMessage {
    source_chain: String,
    message_id: String,
    source_address: String,
    contract_address: FieldElement,
    payload_hash: U256,
}

impl TryFrom<&RouterMessage> for StarknetMessage {
    type Error = Report<Error>;

    fn try_from(msg: &RouterMessage) -> Result<Self, Self::Error> {
        let contract_address = FieldElement::from_str(msg.destination_address.as_str())
            .change_context(Error::InvalidAddress)?;

        Ok(StarknetMessage {
            source_chain: msg.cc_id.source_chain.to_string(),
            message_id: msg.cc_id.message_id.to_string(),
            source_address: msg.source_address.to_string(),
            contract_address,
            payload_hash: U256::from(msg.payload_hash),
        })
    }
}

impl Tokenizable for StarknetMessage {
    fn from_token(token: ethers_core::abi::Token) -> Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        todo!();
    }

    fn into_token(self) -> Token {
        let contract_address_bytes = self.contract_address.to_bytes_be().to_vec();

        Token::Tuple(vec![
            Token::String(self.source_chain),
            Token::String(self.message_id),
            Token::String(self.source_address),
            Token::FixedBytes(contract_address_bytes),
            Token::Uint(self.payload_hash),
        ])
    }
}
