use error_stack::Report;

use crate::{AleoValue, Error};

#[derive(Debug, Clone)]
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
        let source_chain = <&str>::from(&self.cc_id.source_chain);
        let source_address: Vec<u16> = self
            .source_address
            .as_str()
            .as_bytes()
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    ((chunk[0] as u16) << 8) | (chunk[1] as u16)
                } else {
                    (chunk[0] as u16) << 8
                }
            })
            .collect();

        let destination_address_hex: Vec<u8> = hex::decode(self.destination_address.as_str())
            .map_err(|e| {
                Report::new(Error::AleoGateway(format!(
                    "Failed to decode destination address: {}",
                    e
                )))
            })?;

        let message_id: Vec<String> = self
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

        let res = format!(
            r#"{{source_chain: [{}], message_id: [{}], source_address: [{}], contract_address: [{}], payload_hash: [{}]}}"#,
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
            source_address
                .iter()
                .take(source_address.len() - 1)
                .map(|c| format!("{}u16, ", c))
                .chain(source_address.last().map(|c| format!("{}u16", c)))
                .chain(std::iter::repeat(", 0u16".to_string()).take(32 - source_address.len()))
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
                .chain(
                    std::iter::repeat(", 0u8".to_string()).take(20 - destination_address_hex.len())
                )
                .collect::<String>(),
            self.payload_hash
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(res)
    }
}
