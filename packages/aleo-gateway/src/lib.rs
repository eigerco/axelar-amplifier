use std::str::FromStr as _;

use aleo_string_encoder::StringEncoder;
use aleo_types::address::Address;
use error_stack::Report;
use snarkvm_cosmwasm::console::network::Network;
use snarkvm_cosmwasm::console::program::ToBits;
use snarkvm_cosmwasm::console::types::Group;
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
    Aleo(#[from] snarkvm_cosmwasm::console::program::Error),
    #[error("Hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("AleoTypes: {0}")]
    AleoTypes(#[from] aleo_types::Error),
    #[error("InvalidSourceChainLength: expected: {expected}, actual: {actual}")]
    InvalidEncodedStringLength { expected: usize, actual: usize },
    #[error("Invalid ascii character")]
    InvalidAscii,
    #[error("StringEncoder: {0}")]
    StringEncoder(#[from] aleo_string_encoder::Error),
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
    #[error("Invalid ProgramID: {program_id}, fail to create program id with error '{error}'")]
    InvalidProgramID {
        program_id: String,
        error: snarkvm_cosmwasm::console::account::Error,
    },
    #[error("ProgramID to aleo address faild: {program_id}, fail to create program id with error '{error}'")]
    ProgramIDToAleoAddress {
        program_id: String,
        error: snarkvm_cosmwasm::console::account::Error,
    },
    #[error("Invalid aleo address: {address}, fail to create program id with error '{error}'")]
    InvalidAleoAddress {
        address: String,
        error: snarkvm_cosmwasm::console::account::Error,
    },
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
    let aleo_value: Vec<bool> =
        snarkvm_cosmwasm::console::program::Value::<N>::from_str(input.as_ref())
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
    let aleo_value: Vec<bool> =
        snarkvm_cosmwasm::console::program::Value::<N>::from_str(input.as_ref())
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

fn aleo_source_chain(name: &str) -> Result<String, Report<Error>> {
    const SOURCE_CHAIN_LEN: usize = 2;
    let source_chain =
        StringEncoder::encode_string(name).map_err(|e| Report::new(Error::from(e)))?;
    let source_chain_len = source_chain.u128_len();
    error_stack::ensure!(
        source_chain_len <= SOURCE_CHAIN_LEN,
        Error::InvalidEncodedStringLength {
            expected: SOURCE_CHAIN_LEN,
            actual: source_chain.u128_len()
        }
    );
    let source_chain = source_chain
        .consume()
        .into_iter()
        .map(|c| format!("{}u128", c))
        .chain(
            std::iter::repeat("0u128".to_string())
                .take(SOURCE_CHAIN_LEN.saturating_sub(source_chain_len)),
        )
        .collect::<Vec<_>>()
        .join(", ");
    Ok(source_chain)
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
                message: _,
            } => todo!(),
            interchain_token_service::HubMessage::ReceiveFromHub {
                source_chain,
                message,
            } => match message {
                interchain_token_service::Message::InterchainTransfer(interchain_transfer) => {
                    // translate to ItsIncomingInterchainTransfer
                    let inner_message = interchain_transfer.to_aleo_string()?;
                    let source_chain = aleo_source_chain(source_chain.as_ref())?;

                    Ok(format!(
                        "{{ inner_message: {inner_message}, source_chain: [{source_chain}] }}"
                    ))
                }
                interchain_token_service::Message::DeployInterchainToken(
                    deploy_interchain_token,
                ) => {
                    let source_chain = aleo_source_chain(source_chain.as_ref())?;

                    let inner_message = deploy_interchain_token.to_aleo_string()?;

                    Ok(format!(
                        "{{ inner_message: {inner_message}, source_chain: [{source_chain}] }}"
                    ))
                }
                interchain_token_service::Message::LinkToken(link_token) => {
                    link_token.to_aleo_string()
                }
            },
            interchain_token_service::HubMessage::RegisterTokenMetadata(
                _register_token_metadata,
            ) => todo!(),
        }
    }
}

impl AleoValue for interchain_token_service::InterchainTransfer {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let token_id: [u8; 32] = self.token_id.into();
        let its_token_id = [
            u128::from_be_bytes(token_id[0..16].try_into().unwrap()),
            u128::from_be_bytes(token_id[16..32].try_into().unwrap()),
        ];

        let source_address =
            StringEncoder::encode_string(format!("{}", self.source_address).as_str())
                .map_err(|_| {
                    Report::new(Error::AleoGateway(
                        "Failed to encode source address".to_string(),
                    ))
                })?
                .buf
                .iter()
                .map(|byte| format!("{}u128", byte))
                .chain(std::iter::repeat("0u128".to_string()))
                .take(6)
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
            "{{ its_token_id: [{}u128, {}u128], source_address: [{}], destination_address: {}, amount: {}u128 }}",
            its_token_id[0], its_token_id[1], source_address, destination_address, amount
        );

        Ok(output)
    }
}

impl AleoValue for interchain_token_service::DeployInterchainToken {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let token_id: [u8; 32] = self.token_id.into();
        let its_token_id = [
            u128::from_be_bytes(token_id[0..16].try_into().unwrap()),
            u128::from_be_bytes(token_id[16..32].try_into().unwrap()),
        ];

        // TODO: use less strings
        let name = StringEncoder::encode_string(&self.name)
            .map_err(|_| Report::new(Error::AleoGateway("Failed to encode name".to_string())))?
            .buf
            .iter()
            .map(|byte| format!("{}u128", byte))
            .take(1)
            .collect::<String>();

        let symbol = StringEncoder::encode_string(&self.symbol)
            .map_err(|_| Report::new(Error::AleoGateway("Failed to encode symbol".to_string())))?
            .buf
            .iter()
            .map(|byte| format!("{}u128", byte))
            .take(1)
            .collect::<String>();

        let minter = self
            .minter
            .as_ref()
            .map_or(Ok(Address::default()), Address::try_from)
            .map_err(|_| {
                Report::new(Error::AleoGateway(
                    "Failed to parse minter address".to_string(),
                ))
            })?;

        let output = format!(
            "{{ its_token_id: [{}u128, {}u128], name: {name}, symbol: {symbol}, decimals: {}u8, minter: {minter} }}",
            its_token_id[0], its_token_id[1], self.decimals
        );

        Ok(output)
    }
}

impl AleoValue for interchain_token_service::LinkToken {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        todo!("not implemented yet. We need to implement this to support Custom Tokens")
    }
}
