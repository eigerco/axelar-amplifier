use std::str::FromStr as _;

use aleo_string_encoder::string_encoder::StringEncoder;
use aleo_types::address::Address;
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
    StringEncoder(#[from] aleo_string_encoder::string_encoder::Error),
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
        error: snarkvm_cosmwasm::account::Error,
    },
    #[error("ProgramID to aleo address faild: {program_id}, fail to create program id with error '{error}'")]
    ProgramIDToAleoAddress {
        program_id: String,
        error: snarkvm_cosmwasm::account::Error,
    },
    #[error("Invalid aleo address: {address}, fail to create program id with error '{error}'")]
    InvalidAleoAddress {
        address: String,
        error: snarkvm_cosmwasm::account::Error,
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
                source_chain,
                message,
            } => match message {
                interchain_token_service::Message::InterchainTransfer(interchain_transfer) => {
                    interchain_transfer.to_aleo_string()
                }
                interchain_token_service::Message::DeployInterchainToken(
                    deploy_interchain_token,
                ) => {
                    const SOURCE_CHAIN_LEN: usize = 2;
                    let source_chain = StringEncoder::encode_string(source_chain.as_ref())
                        .map_err(|e| Report::new(Error::from(e)))?;
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
        let token_id = [
            u128::from_le_bytes(self.token_id.0[0..16].try_into().unwrap()),
            u128::from_le_bytes(self.token_id.0[16..32].try_into().unwrap()),
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

        let output = format!(
            "{{ its_token_id: [{}u128, {}u128], name: {}, symbol: {}, decimals: {}u8 }}",
            token_id[0], token_id[1], name, symbol, self.decimals
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
    use sha3::Digest as _;
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

    #[test]
    fn bar() {
        let msg_hash = "2fdcbfc3853f71262a91ee9b837c18a4a7df711eb563eea38e5d3f501f4157be";
        let msg_hash_bytes = hex::decode(msg_hash).unwrap();

        let message = router_api::Message {
            cc_id: router_api::CrossChainId::new(
                "axelar",
                "0x32a1fb889d3f6c9f92ff0b152863d36832e53e0cf9429841d8766e64c2e2d408-503643",
            )
            .unwrap(),
            source_address: "axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4"
                .parse()
                .unwrap(),
            destination_chain: "aleo-2".parse().unwrap(),
            destination_address: "aleo1ymrcwun5g9z0un8dqgdln7l3q77asqr98p7wh03dwgk4yfltpqgq9efvfz"
                .parse()
                .unwrap(),
            payload_hash: [
                47, 220, 191, 195, 133, 63, 113, 38, 42, 145, 238, 155, 131, 124, 24, 164, 167,
                223, 113, 30, 181, 99, 238, 163, 142, 93, 63, 80, 31, 65, 87, 190,
            ],
        };

        let aleo_message = Message::try_from(&message).unwrap();
        println!("Aleo message: {:?}", aleo_message.to_aleo_string());
    }

    fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
        bytes
            .iter()
            .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1 == 1))
            .collect()
    }

    fn translate_hash(hash_bytes: &[u8; 32]) {
        let reverse_hash: Vec<u8> = hash_bytes.iter().map(|b| b.reverse_bits()).collect();
        let keccak_bits: Vec<bool> = bytes_to_bits(&reverse_hash);

        let group = <snarkvm_cosmwasm::network::TestnetV0>::hash_to_group_bhp256(&keccak_bits)
            .map_err(|e| Report::new(Error::from(e)))
            .unwrap();

        let payload_hash = format!("{group}");
        println!("Payload hash: {payload_hash}");
    }

    #[test]
    fn foobar() {
        let payload = "0000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000000e6176616c616e6368652d66756a6900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000001efcc58108973de6489782e9aca7dc9412e970b6f62fe09dc0853e1530a07414c00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000000a536f6d65546f6b656e39000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003534d3900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let payload_bytes = hex::decode(payload).unwrap();
        let its_message = interchain_token_service::HubMessage::abi_decode(&payload_bytes).unwrap();

        let aleo_string = its_message.to_aleo_string().unwrap();

        println!("aleo_string: '{aleo_string}'");

        let hash = crate::hash::<&str, snarkvm_cosmwasm::network::TestnetV0>(&aleo_string).unwrap();
        println!("hash: {hash:?}");
        translate_hash(&hash);

        // let message =
    }

    #[test]
    fn foo() {
        let payload = "0000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000000e6176616c616e6368652d66756a6900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000001d1029f450ce147882104b0e4e1af4d5b9bd9fb71b806b353d4919fb1fa88637500000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000000a536f6d65546f6b656e35000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003534d3500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let payload_bytes = hex::decode(payload).unwrap();
        let its_message = interchain_token_service::HubMessage::abi_decode(&payload_bytes).unwrap();

        let aleo_string = its_message.to_aleo_string().unwrap();

        let aleo_value =
            snarkvm_cosmwasm::program::Value::<TestnetV0>::from_str(aleo_string.as_ref());

        assert!(
            aleo_value.is_ok(),
            "aleo_string: {aleo_string:?}\naleo_value: {aleo_value:?}"
        );

        println!("aleo_string: {aleo_string}");
        println!("aleo_value: {aleo_value:?}");
        let hash = crate::hash::<&str, snarkvm_cosmwasm::network::TestnetV0>(&aleo_string).unwrap();
        println!("aleo payload hash: {hash:?}");

        let hash = "2fdcbfc3853f71262a91ee9b837c18a4a7df711eb563eea38e5d3f501f4157be";
        let hash_bytes = hex::decode(hash).unwrap();
        println!("expected hash: {hash_bytes:?}");

        // let payload_hash: [u8; 32] = sha3::Keccak256::digest(payload_bytes).into();
        // let s: String = hex::encode(payload_hash);
        // println!("abi payload hash: {s}");
        //
        // let error_hash: [u8; 32] = [
        //     56, 57, 163, 74, 223, 238, 74, 184, 131, 74, 146, 110, 120, 95, 70, 89, 89, 39, 196,
        //     17, 198, 105, 70, 136, 173, 145, 116, 45, 183, 138, 123, 191,
        // ];
        // let s: String = hex::encode(error_hash);
        // println!("error hash: {s:?}");
        //
        // let payload = "0000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000006616c656f2d32000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000001d1029f450ce147882104b0e4e1af4d5b9bd9fb71b806b353d4919fb1fa88637500000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000120000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000000a536f6d65546f6b656e35000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003534d3500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        // let payload_bytes = hex::decode(payload).unwrap();
        // let payload_hash: [u8; 32] = sha3::Keccak256::digest(payload_bytes).into();
        // let s: String = hex::encode(payload_hash);
        // println!("abi payload hash: {s}");
        //
        // let error_hash: [u8; 32] = [
        //     56, 57, 163, 74, 223, 238, 74, 184, 131, 74, 146, 110, 120, 95, 70, 89, 89, 39, 196,
        //     17, 198, 105, 70, 136, 173, 145, 116, 45, 183, 138, 123, 191,
        // ];
        // let s: String = hex::encode(error_hash);
        // println!("expected: {s:?}");
        //
        // let error_hash: [u8; 32] = [
        //     47, 220, 191, 195, 133, 63, 113, 38, 42, 145, 238, 155, 131, 124, 24, 164, 167, 223,
        //     113, 30, 181, 99, 238, 163, 142, 93, 63, 80, 31, 65, 87, 190,
        // ];
        // let s: String = hex::encode(error_hash);
        // println!("found: {s:?}");
    }
}
