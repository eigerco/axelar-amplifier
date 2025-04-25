use aleo_utils::string_encoder::StringEncoder;
use error_stack::{ensure, Report};
use serde::Deserialize;
use snarkvm_cosmwasm::network::Network;

use crate::{AleoValue, Error};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    bytes
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1 == 1))
        .collect()
}

impl AleoValue for Message {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        const SOURCE_CHAIN_LEN: usize = 2;
        let source_chain = StringEncoder::encode_string(self.cc_id.source_chain.as_ref())
            .map_err(|e| Report::new(Error::from(e)))?;
        let source_chain_len = source_chain.u128_len();
        ensure!(
            source_chain_len <= SOURCE_CHAIN_LEN,
            Error::InvalidEncodedStringLength {
                expected: SOURCE_CHAIN_LEN,
                actual: source_chain.u128_len()
            }
        );

        const MESSAGE_ID_LEN: usize = 8;
        let message_id = StringEncoder::encode_string(self.cc_id.message_id.as_str())
            .map_err(|e| Report::new(Error::from(e)))?;
        let message_id_len = message_id.u128_len();
        ensure!(
            message_id_len <= MESSAGE_ID_LEN,
            Error::InvalidEncodedStringLength {
                expected: MESSAGE_ID_LEN,
                actual: message_id.u128_len()
            }
        );

        const SOURCE_ADDRESS_LEN: usize = 4;
        let source_address = StringEncoder::encode_string(self.source_address.as_str())
            .map_err(|e| Report::new(Error::from(e)))?;

        let source_address_len = source_address.u128_len();
        ensure!(
            source_address_len <= SOURCE_ADDRESS_LEN,
            Error::InvalidEncodedStringLength {
                expected: SOURCE_ADDRESS_LEN,
                actual: source_address.u128_len()
            }
        );

        const CONTRACT_ADDRESS_LEN: usize = 4;
        let contract_address = StringEncoder::encode_string(self.destination_address.as_str())
            .map_err(|e| Report::new(Error::from(e)))?;

        let contract_address_len = contract_address.u128_len();
        ensure!(
            contract_address_len <= CONTRACT_ADDRESS_LEN,
            Error::InvalidEncodedStringLength {
                expected: CONTRACT_ADDRESS_LEN,
                actual: contract_address.u128_len()
            }
        );

        // The payload hash is a 32 byte array, which is a 256 bit hash.
        // (for messages from Aleo this will happen in the relayer)
        // The group values of Aleo are ~256bits, so in aleo we will only use bhp256(keccak256) hashes.
        // The result of bhp256 is a group element, which comes from Aleo.
        // We will store it in cosmos 256 bits variables just for convenience.
        let reverse_hash: Vec<u8> = self.payload_hash.iter().map(|b| b.reverse_bits()).collect();
        let keccak_bits: Vec<bool> = bytes_to_bits(&reverse_hash);

        let group = <snarkvm_cosmwasm::network::TestnetV0>::hash_to_group_bhp256(&keccak_bits)
            .map_err(|e| Report::new(Error::from(e)))?;

        let payload_hash = format!("{group}");

        let res = format!(
            r#"{{source_chain: [{}], message_id: [{}], source_address: [{}], contract_address: [{}], payload_hash: {} }}"#,
            source_chain
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c))
                .chain(
                    std::iter::repeat("0u128".to_string())
                        .take(SOURCE_CHAIN_LEN.saturating_sub(source_chain_len))
                )
                .collect::<Vec<_>>()
                .join(", "),
            message_id
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c))
                .chain(
                    std::iter::repeat("0u128".to_string())
                        .take(MESSAGE_ID_LEN.saturating_sub(message_id_len))
                )
                .collect::<Vec<_>>()
                .join(", "),
            source_address
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c))
                .chain(
                    std::iter::repeat("0u128".to_string())
                        .take(SOURCE_ADDRESS_LEN.saturating_sub(source_address_len))
                )
                .collect::<Vec<_>>()
                .join(", "),
            contract_address
                .consume()
                .into_iter()
                .map(|c| format!("{}u128", c))
                .chain(
                    std::iter::repeat("0u128".to_string())
                        .take(CONTRACT_ADDRESS_LEN.saturating_sub(contract_address_len))
                )
                .collect::<Vec<_>>()
                .join(", "),
            payload_hash
        );

        Ok(res)
    }
}
