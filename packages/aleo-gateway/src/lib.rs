use std::str::FromStr as _;

use aleo_types::address::Address;
use aleo_utils::string_encoder::StringEncoder;
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

// Generics are not used in the code because of this issue:
// https://github.com/rust-lang/rust/issues/61956
// For this we will use this const variables, just to be easy for as to adapt during development.
// TODO: When our solution is ready, we will need to rethink it.
pub const GROUP_SIZE: usize = 14;
pub const GROUPS: usize = 2;

type Array2D<T> = [[T; GROUP_SIZE]; GROUPS];

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

impl AleoValue for interchain_token_service::HubMessage {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        // We need to support
        // 1. InterchainTransfer
        // 2. DeployInterchainToken
        // 3. LinkToken

        match self {
            interchain_token_service::HubMessage::SendToHub {
                destination_chain: _,
                message,
            } => match message {
                interchain_token_service::Message::InterchainTransfer(interchain_transfer) => {
                    interchain_transfer.to_aleo_string()
                }
                interchain_token_service::Message::DeployInterchainToken(
                    deploy_interchain_token,
                ) => deploy_interchain_token.to_aleo_string(),
                interchain_token_service::Message::LinkToken(link_token) => {
                    link_token.to_aleo_string()
                }
            },
            interchain_token_service::HubMessage::ReceiveFromHub {
                source_chain: _,
                message: _,
            } => todo!(),
            interchain_token_service::HubMessage::RegisterTokenMetadata(
                _register_token_metadata,
            ) => todo!(),
        }
    }
}

impl AleoValue for interchain_token_service::InterchainTransfer {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let token_id: String = self
            .token_id
            .0
            .iter()
            .map(|byte| format!("{}u8", byte))
            .collect::<Vec<_>>()
            .join(", ");

        let source_address = StringEncoder::encode_bytes(&self.source_address)
            .map_err(|_| {
                Report::new(Error::AleoGateway(
                    "Failed to encode source address".to_string(),
                ))
            })?
            .buf
            .iter()
            .map(|byte| format!("{}u128", byte))
            .take(16)
            .collect::<Vec<_>>()
            .join(", ");

        let destination_address = Address::try_from(&self.destination_address).map_err(|_| {
            Report::new(Error::AleoGateway(
                "Failed to parse destination address".to_string(),
            ))
        })?;

        let amount: u128 = self.amount.to_string().parse().map_err(|_| {
            Report::new(Error::AleoGateway(
                "Failed to parse amount into u128".to_string(),
            ))
        })?;

        let output = format!(
            "{{ token_id: [{}], source_address: [{}], destination_address: {}, amount: {}u128 }}",
            token_id, source_address, destination_address, amount
        );

        Ok(output)
    }
}

impl AleoValue for interchain_token_service::DeployInterchainToken {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let token_id: String = self
            .token_id
            .0
            .iter()
            .map(|byte| format!("{}u8", byte))
            .collect::<Vec<_>>()
            .join(", ");

        let name = StringEncoder::encode_string(&self.name)
            .map_err(|_| Report::new(Error::AleoGateway("Failed to encode name".to_string())))?
            .buf
            .iter()
            .map(|byte| format!("{}u128", byte))
            .take(1)
            .collect::<Vec<_>>()
            .join(", ");

        let symbol = StringEncoder::encode_string(&self.symbol)
            .map_err(|_| Report::new(Error::AleoGateway("Failed to encode symbol".to_string())))?
            .buf
            .iter()
            .map(|byte| format!("{}u128", byte))
            .take(1)
            .collect::<Vec<_>>()
            .join(", ");

        let output = format!(
            "{{ token_id: [{}], name: [{}], symbol: [{}], decimals: {}u8 }}",
            token_id, name, symbol, self.decimals
        );

        Ok(output)
    }
}

impl AleoValue for interchain_token_service::LinkToken {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        todo!("not implemented yet")
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use axelar_wasm_std::nonempty::HexBinary;
    use interchain_token_service::TokenId;
    use snarkvm_cosmwasm::network::TestnetV0;

    use super::*;

    #[test]
    fn translate_deploy_interchain_token() {
        let deploy_interchain_token = interchain_token_service::DeployInterchainToken {
            token_id: TokenId([0u8; 32]),
            name: axelar_wasm_std::nonempty::String::from_str("Test Token").unwrap(),
            symbol: axelar_wasm_std::nonempty::String::from_str("TT").unwrap(),
            decimals: 18,
            minter: None,
        };

        let aleo_string = deploy_interchain_token.to_aleo_string().unwrap();

        let aleo_value =
            snarkvm_cosmwasm::program::Value::<TestnetV0>::from_str(aleo_string.as_ref());

        assert!(
            aleo_value.is_ok(),
            "aleo_string: {aleo_string:?}\naleo_value: {aleo_value:?}"
        );
    }

    #[test]
    fn translate_interchain_transfer() {
        let aleo_address = aleo_types::address::Address::default();
        let aleo_address_bytes = aleo_address.to_bytes();

        let amount: cosmwasm_std::Uint256 = cosmwasm_std::Uint256::from(100u128);
        let interchain_transfer = interchain_token_service::InterchainTransfer {
            token_id: TokenId([0u8; 32]),
            source_address: HexBinary::try_from(vec![1, 2, 3]).unwrap(),
            destination_address: HexBinary::try_from(aleo_address_bytes).unwrap(),
            amount: axelar_wasm_std::nonempty::Uint256::try_from(amount).unwrap(),
            data: None,
        };
        let aleo_string = interchain_transfer.to_aleo_string().unwrap();

        let aleo_value =
            snarkvm_cosmwasm::program::Value::<TestnetV0>::from_str(aleo_string.as_ref());

        assert!(
            aleo_value.is_ok(),
            "aleo_string: {aleo_string:?}\naleo_value: {aleo_value:?}"
        );
    }
}
