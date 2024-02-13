use axelar_wasm_std::voting::Vote;
use base64::Engine as _;
use borsh::{BorshDeserialize, BorshSerialize};

use base64::{self, engine::general_purpose};
use tracing::warn;

use crate::handlers::solana_verify_msg::Message;

use super::{json_rpc::EncodedConfirmedTransactionWithStatusMeta, pub_key_wrapper::PubkeyWrapper};

// Gateway program logs.
// Logged when the Gateway receives an outbound message.
#[derive(Debug, PartialEq, BorshDeserialize, BorshSerialize, Clone)]
#[repr(u8)]
enum GatewayEvent {
    CallContract {
        // Message sender.
        sender: PubkeyWrapper,
        destination_chain: Vec<u8>,
        destination_address: Vec<u8>,
        payload: Vec<u8>,
        payload_hash: [u8; 32],
    },
}

impl GatewayEvent {
    // Try to parse a [`CallContractEvent`] out of a Solana program log line.
    fn parse_log(log: &String) -> Option<Self> {
        let cleaned_input = log
            .trim()
            .trim_start_matches("Program data:")
            .split_whitespace()
            .flat_map(decode_base64)
            .next()?;
        borsh::from_slice(&cleaned_input).ok()
    }
}

impl PartialEq<&Message> for GatewayEvent {
    fn eq(&self, msg: &&Message) -> bool {
        match self {
            GatewayEvent::CallContract {
                sender: _,
                destination_chain,
                destination_address,
                payload: _,
                payload_hash,
            } => {
                let event_dest_addr = String::from_utf8(destination_address.to_owned());
                let event_dest_chain = String::from_utf8(destination_chain.to_owned());

                event_dest_addr.is_ok()
                    && event_dest_chain.is_ok()
                    && event_dest_addr.unwrap() == msg.destination_address
                    && event_dest_chain.unwrap() == msg.destination_chain.to_string()
                    && payload_hash.to_owned() == msg.payload_hash
            }
        }
    }
}

#[inline]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}

pub fn verify_message(
    source_gateway_address: &String, // TODO: check if sender is source_gateway_address
    tx: &EncodedConfirmedTransactionWithStatusMeta,
    message: &Message,
) -> Vote {
    if let None = tx.meta.log_messages {
        // TODO: Log error/warn, because this event should contain log msgs?
        return Vote::NotFound;
    }

    let program_data = tx.meta.log_messages.as_ref().unwrap();
    let gw_event_parsed: Option<GatewayEvent> = program_data
        .into_iter()
        // TODO: Will find_map work with multiple msgs in transaction?
        .find_map(|program_log| GatewayEvent::parse_log(program_log));

    // let prog_data_base64_borsh = get_program_data_from_log(tx.meta.log_messages.as_ref());
    // let prog_data = decode_program_data(prog_data_base64_borsh.clone()).unwrap(); // TODO: Should

    if gw_event_parsed.is_none() {
        // TODO: Log error/warn, because this event should be parsed
        warn!("failed to parse the event");
        return Vote::FailedOnChain;
    }

    //NOTE: first signagure is always tx_id
    let verified = gw_event_parsed.clone().unwrap() == message
        && tx.transaction.signatures[0] == message.tx_id
        && tx
            .transaction
            .message
            .account_keys
            .contains(source_gateway_address);

    match verified {
        true => Vote::SucceededOnChain,
        false => Vote::FailedOnChain,
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::solana::json_rpc::{
        SolInstruction, SolMessage, Transaction, UiTransactionStatusMeta,
    };
    use connection_router::state::ChainName;
    use solana_program::pubkey::Pubkey;

    use super::*;

    #[test]
    fn should_verify_msg_if_correct() {
        let (source_gateway_address, tx, msg) = get_matching_msg_and_tx_block();
        assert_eq!(
            Vote::SucceededOnChain,
            verify_message(&source_gateway_address, &tx, &msg)
        );
    }

    fn get_matching_msg_and_tx_block(
    ) -> (String, EncodedConfirmedTransactionWithStatusMeta, Message) {
        // Common fields among tx and message.
        let tx_id = "fake_tx_id".to_string();
        let destination_chain = "eth".to_string();
        let destination_address = "0x0".to_string();
        let payload: Vec<u8> = Vec::new();
        let payload_hash: [u8; 32] = [0; 32];
        let source_gateway_address: String = "sol".to_string();

        let event = gateway::events::GatewayEvent::CallContract {
            sender: Pubkey::from([0; 32]).into(),
            destination_chain: destination_chain.clone().into_bytes(),
            destination_address: destination_address.clone().into_bytes(),
            payload,
            payload_hash,
        };

        let mut event_data = Vec::new();
        event.serialize(&mut event_data).unwrap();
        let event_data_b64 = general_purpose::STANDARD.encode(event_data);
        let mut log_message = "Program data: ".to_string();
        log_message.push_str(&event_data_b64);

        let tx = EncodedConfirmedTransactionWithStatusMeta {
            transaction: Transaction {
                message: SolMessage {
                    instructions: vec![SolInstruction {
                        data: "".to_string(),
                    }],
                    account_keys: vec![source_gateway_address.clone()],
                },
                signatures: vec![tx_id.clone()],
            },
            meta: UiTransactionStatusMeta {
                log_messages: Some(vec![log_message]),
            },
        };

        let message = Message {
            tx_id,
            event_index: 0,
            destination_address: destination_address.clone(),
            destination_chain: ChainName::from_str(&destination_chain).unwrap(),
            source_address: source_gateway_address.clone(),
            payload_hash,
        };

        (source_gateway_address, tx, message)
    }

    #[test]
    fn should_not_verify_msg_if_tx_id_does_not_match() {
        let (source_gateway_address, tx, mut msg) = get_matching_msg_and_tx_block();
        msg.tx_id = "wrong_tx_id".to_string();
        assert_eq!(
            Vote::FailedOnChain,
            verify_message(&source_gateway_address, &tx, &msg)
        );
    }

    #[ignore = "We are not checking the event index in production code."]
    #[test]
    fn should_not_verify_msg_if_event_index_does_not_match() {
        let (gateway_address, tx, mut msg) = get_matching_msg_and_tx_block();
        msg.event_index = rand::random::<u64>();
        assert_eq!(Vote::NotFound, verify_message(&gateway_address, &tx, &msg));
    }
    #[ignore = "We are not checking the source address in the gateway event."]
    #[test]
    fn should_not_verify_msg_if_source_address_does_not_match() {
        let (gateway_address, tx, mut msg) = get_matching_msg_and_tx_block();
        msg.source_address = "bad_address".to_string();
        assert_eq!(Vote::NotFound, verify_message(&gateway_address, &tx, &msg));
    }

    #[test]
    fn should_not_verify_msg_if_destination_chain_does_not_match() {
        let (gateway_address, tx, mut msg) = get_matching_msg_and_tx_block();
        msg.destination_chain = ChainName::from_str("bad_chain").unwrap();
        assert_eq!(
            Vote::FailedOnChain,
            verify_message(&gateway_address, &tx, &msg)
        );
    }

    #[test]
    fn should_not_verify_msg_if_destination_address_does_not_match() {
        let (gateway_address, tx, mut msg) = get_matching_msg_and_tx_block();
        msg.destination_address = "bad_address".to_string();
        assert_eq!(
            Vote::FailedOnChain,
            verify_message(&gateway_address, &tx, &msg)
        );
    }

    #[test]
    fn should_not_verify_msg_if_payload_hash_does_not_match() {
        let (gateway_address, tx, mut msg) = get_matching_msg_and_tx_block();
        msg.payload_hash = [1; 32];
        assert_eq!(
            Vote::FailedOnChain,
            verify_message(&gateway_address, &tx, &msg)
        );
    }
}
