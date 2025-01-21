use std::str::FromStr as _;
use snarkvm_cosmwasm::network::Network;
// use cosmwasm_std::Uint256;
use error_stack::{ensure, Report};
use snarkvm_cosmwasm::program::ToBits as _;

use crate::string_encoder::StringEncoder;
use crate::{AleoValue, Error};

/*
    struct Message {
        // ascii encoded chain name
        source_chain: [u128; 2],
        // TODO: unit test all valid message_id formats
        message_id: [u128; 8],
        // TODO: unit test few valid source addresses
        source_address: [u128; 4],
        // detination contract on aleo
        contract_address: [u128; 4], // This is the program name
        // hash of the payload
        payload_hash: [u8; 32],
    }
*/

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub cc_id: router_api::CrossChainId,
    pub source_address: String,
    pub destination_chain: router_api::ChainName,
    pub destination_address: String,
    pub payload_hash: [u8; 32], // The hash of the payload, send from the relayer of the source chain
}

impl TryFrom<&router_api::Message> for Message {
    type Error = Report<Error>;

    fn try_from(value: &router_api::Message) -> Result<Self, Self::Error> {
        Ok(Message {
            cc_id: value.cc_id.clone(),
            source_address: value.source_address.to_string(),
            destination_chain: value.destination_chain.clone(),
            destination_address: value.destination_address.to_string(),
            payload_hash: value.payload_hash,
        })
    }
}

impl AleoValue for Message {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        const SOURCE_CHAIN_LEN: usize = 2;
        let source_chain = StringEncoder::encode_string(self.cc_id.source_chain.as_ref())?;
        let source_chain_len = source_chain.u128_len();
        ensure!(
            source_chain_len <= SOURCE_CHAIN_LEN,
            Error::InvalidEncodedStringLength {
                expected: SOURCE_CHAIN_LEN,
                actual: source_chain.u128_len()
            }
        );

        const MESSAGE_ID_LEN: usize = 8;
        let message_id = StringEncoder::encode_string(self.cc_id.message_id.as_str())?;
        let message_id_len = message_id.u128_len();
        ensure!(
            message_id_len <= MESSAGE_ID_LEN,
            Error::InvalidEncodedStringLength {
                expected: MESSAGE_ID_LEN,
                actual: message_id.u128_len()
            }
        );

        const SOURCE_ADDRESS_LEN: usize = 4;
        let source_address = StringEncoder::encode_string(self.source_address.as_str())?;
        let source_address_len = source_address.u128_len();
        ensure!(
            source_address_len <= SOURCE_ADDRESS_LEN,
            Error::InvalidEncodedStringLength {
                expected: SOURCE_ADDRESS_LEN,
                actual: source_address.u128_len()
            }
        );

        const CONTRACT_ADDRESS_LEN: usize = 4;
        let contract_address = StringEncoder::encode_string(self.destination_address.as_str())?;
        let contract_address_len = contract_address.u128_len();
        ensure!(
            contract_address_len <= CONTRACT_ADDRESS_LEN,
            Error::InvalidEncodedStringLength {
                expected: CONTRACT_ADDRESS_LEN,
                actual: contract_address.u128_len()
            }
        );

        let payload_hash = cosmwasm_std::Uint256::from_le_bytes(self.payload_hash);

        let res = format!(
            r#"{{source_chain: [{}], message_id: [{}], source_address: [{}], contract_address: [{}], payload_hash: {}group }}"#,
            source_chain
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c as u128))
                .chain(
                    std::iter::repeat("0u128".to_string())
                        .take(SOURCE_CHAIN_LEN - source_chain_len)
                )
                .collect::<Vec<_>>()
                .join(", "),
            message_id
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c as u128))
                .chain(std::iter::repeat("0u128".to_string()).take(MESSAGE_ID_LEN - message_id_len))
                .collect::<Vec<_>>()
                .join(", "),
            source_address
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c as u128))
                .chain(
                    std::iter::repeat("0u128".to_string())
                        .take(SOURCE_ADDRESS_LEN - source_address_len)
                )
                .collect::<Vec<_>>()
                .join(", "),
            contract_address
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c as u128))
                .chain(
                    std::iter::repeat("0u128".to_string())
                        .take(CONTRACT_ADDRESS_LEN - contract_address_len)
                )
                .collect::<Vec<_>>()
                .join(", "),
            payload_hash
        );

        Ok(res)
    }
}
