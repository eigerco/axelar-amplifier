use std::{fmt::Display, str::FromStr as _};

use aleo_types::{address::Address, program::Program};
use bitvec::prelude::*;
use cosmwasm_std::{Addr, HexBinary, Uint128};
use error_stack::{bail, Report, ResultExt};
use multisig::verifier_set::VerifierSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid program name: {0}")]
    InvalidProgramName(String),
}

#[derive(Debug, Clone)]
pub struct Hash(pub [u8; 32]);

#[derive(Debug, Clone)]
pub struct Message {
    pub buffer: String,
}

pub struct WeightedSigner {
    signer: Address,
    weight: Uint128,
}

pub struct WeightedSigners {
    signers: Vec<WeightedSigner>, // TODO: [WeightedSigner; 32],
    threshold: Uint128,
    nonce: [u64; 4],
}

pub struct Proof {
    signers: WeightedSigners,
    signatures: Vec<u8>,
}

impl TryFrom<&VerifierSet> for WeightedSigners {
    type Error = Report<Error>;

    fn try_from(value: &VerifierSet) -> Result<Self, Self::Error> {
        let signers = value
            .signers
            .iter()
            .map(|(address, signer)| WeightedSigner {
                signer: Address::from_str(address.as_str()).unwrap(),
                weight: signer.weight,
            })
            .collect::<Vec<_>>();

        let threshold = value.threshold;
        let nonce = [0, 0, 0, value.created_at];

        Ok(WeightedSigners {
            signers,
            threshold,
            nonce,
        })
    }
}

impl Display for WeightedSigners {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let signers = self
            .signers
            .iter()
            .map(|signer| format!("{{signer: {}, weight: {}}}", signer.signer.0, signer.weight))
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            r#"{{signers: [{}], threshold: {}u128, nonce: [{}u64, {}u64], {}u64, {}u64}}"#,
            signers, self.threshold, self.nonce[0], self.nonce[1], self.nonce[2], self.nonce[3]
        )
    }
}

impl Display for Proof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let signers = self.signers.to_string();
        let signatures = self
            .signatures
            .iter()
            .map(|b| format!("{}u8", b))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, r#"{{signers: {}, signatures: [{}]}}"#, signers, signatures)
    }
}

impl TryFrom<&router_api::Message> for Message {
    type Error = Report<Error>;

    fn try_from(value: &router_api::Message) -> Result<Self, Self::Error> {
        let source_chain = <&str>::from(&value.cc_id.source_chain);
        let source_address_hex: Vec<u8> = hex::decode(value.source_address.as_str()).unwrap();
        let destination_address_hex: Vec<u8> =
            hex::decode(value.destination_address.as_str()).unwrap();
        let message_id: Vec<String> = value
            .cc_id
            .message_id
            .chars()
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|chunk| {
                let high = chunk.get(0).map_or(0, |&c| c as u16);
                let low = chunk.get(1).map_or(0, |&c| c as u16);
                let value = (high << 8) | low;
                format!("{}u16", value)
            })
            .collect();
        let message_id = message_id.join(", ");

        let buffer = format!(
            r#"{{source_chain: [{}], message_id: [{}], source_address: [{}], destination_address: [{}], payload_hash: [{}]}}"#,
            source_chain
                .chars()
                .take(source_chain.len() - 1)
                .map(|c| format!("{}u8, ", c as u8))
                .chain(
                    source_chain
                        .chars()
                        .last()
                        .map(|c| format!("{}u8", c as u8))
                )
                .chain(std::iter::repeat(", 0u8".to_string()).take(32 - source_chain.len()))
                .collect::<String>(),
            message_id,
            source_address_hex
                .iter()
                .take(source_address_hex.len() - 1)
                .map(|c| format!("{}u8, ", c))
                .chain(source_address_hex.last().map(|c| format!("{}u8", c)))
                .chain(std::iter::repeat(", 0u8".to_string()).take(20 - source_chain.len()))
                .collect::<String>(),
            destination_address_hex
                .iter()
                .take(destination_address_hex.len() - 1)
                .map(|c| format!("{}u8, ", c))
                .chain(
                    destination_address_hex
                        .iter()
                        .last()
                        .map(|c| format!("{}u8", c))
                )
                .chain(std::iter::repeat(", 0u8".to_string()).take(20 - source_chain.len()))
                .collect::<String>(),
            value
                .payload_hash
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(Message { buffer })
    }
}

pub struct Messages(Vec<Message>);

impl From<Vec<Message>> for Messages {
    fn from(v: Vec<Message>) -> Self {
        Messages(v)
    }
}

impl Messages {
    pub fn messages_approval_hash(&self) -> Result<[u8; 32], Error> {
        Ok([0u8; 32])
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{Addr, HexBinary, Uint128};
    use router_api::CrossChainId;
    use router_api::Message as RouterMessage;

    use super::*;

    #[test]
    fn router_message_to_gateway_message() {
        let source_chain = "chain0";
        let message_id = "au14zeyyly2s2nc8f4vze5u2gs27uyjv72qds66cvre3tlwrewqdurqpsj839";
        let source_address = "52444f1835Adc02086c37Cb226561605e2E1699b";
        let destination_chain = "chain1";
        let payload_hash = "8c3685dc41c2eca11426f8035742fb97ea9f14931152670a5703f18fe8b392f0";
        let destination_address = "a4f10f76b86e01b98daf66a3d02a65e14adb0767";

        let router_messages = RouterMessage {
            cc_id: CrossChainId::new(source_chain, message_id).unwrap(),
            source_address: source_address.parse().unwrap(),
            destination_address: destination_address.parse().unwrap(),
            destination_chain: destination_chain.parse().unwrap(),
            payload_hash: HexBinary::from_hex(payload_hash)
                .unwrap()
                .to_array::<32>()
                .unwrap(),
        };

        let gateway_message = Message::try_from(&router_messages).unwrap();
        println!("{:?}", gateway_message.buffer);
    }
}
