use axelar_wasm_std::voting::Vote;

use super::events::contract_call::ContractCallEvent;
use crate::handlers::starknet_verify_msg::Message;

/// Attempts to fetch the tx provided in `axl_msg.tx_id`.
/// If successful, extracts and parses the ContractCall event
/// and compares it to the message from the relayer (via PollStarted event).
/// Also checks if the source_gateway_address with which
/// the voting verifier has been instantiated is the same address from
/// which the ContractCall event is coming.
pub fn verify_msg(
    starknet_event: &ContractCallEvent,
    msg: &Message,
    source_gateway_address: &str,
) -> Vote {
    dbg!(starknet_event);
    dbg!(msg);
    dbg!(source_gateway_address);
    if *starknet_event == *msg && starknet_event.from_contract_addr == source_gateway_address {
        Vote::SucceededOnChain
    } else {
        Vote::NotFound
    }
}

impl PartialEq<Message> for ContractCallEvent {
    fn eq(&self, axl_msg: &Message) -> bool {
        axl_msg.source_address == self.source_address
            && axl_msg.destination_chain == self.destination_chain
            && axl_msg.destination_address == self.destination_address
            && axl_msg.payload_hash == self.payload_hash
    }
}

#[cfg(test)]
mod tests {
    use ethers::types::H256;
    use starknet_core::utils::{parse_cairo_short_string, starknet_keccak};
    use starknet_providers::jsonrpc::HttpTransport;

    use crate::starknet::events::contract_call::ContractCallEvent;
    use crate::starknet::json_rpc::MockStarknetClient;

    // "hello" as payload
    // "hello" as destination address
    // "some_contract_address" as source address
    // "destination_chain" as destination_chain
    fn mock_valid_event() -> ContractCallEvent {
        let from_contract_addr =
            parse_cairo_short_string(&starknet_keccak("some_contract_address".as_bytes())).unwrap();
        ContractCallEvent {
            from_contract_addr,
            destination_address: String::from("hello"),
            destination_chain: String::from("destination_chain"),
            source_address: String::from(
                "0x00b3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca",
            ),
            payload_hash: H256::from_slice(&[
                28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123, 86, 217,
                81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
            ]),
        }
    }

    fn shoud_verify_event() {
        // let mut mock_client = MockStarknetClient::<HttpTransport>::new();
        // mock_client
        //     .expect_get_event_by_hash()
        //     .returning(|_| Ok(Some(("some_tx_hash".to_owned(),
        // mock_valid_event()))));
        //
        // let verifier = RPCMessageVerifier::new("doesnt_matter");
        // verifier.client = mock_client;
    }
}
