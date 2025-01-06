use axelar_wasm_std::hash::Hash;
use cosmwasm_std::Uint64;
use multisig::verifier_set::VerifierSet;
use router_api::{ChainName, CrossChainId};

use crate::payload::{Payload, PayloadId};

pub enum Event {
    ProofUnderConstruction {
        destination_chain: ChainName,
        payload_id: PayloadId,
        multisig_session_id: Uint64,
        msg_ids: Vec<CrossChainId>,
    },
    DebugEvent {
        domain_separator: Hash,
        verifier_set: VerifierSet,
        payload: Payload,
    },
}

impl From<Event> for cosmwasm_std::Event {
    fn from(other: Event) -> Self {
        match other {
            Event::ProofUnderConstruction {
                destination_chain,
                payload_id,
                multisig_session_id,
                msg_ids,
            } => cosmwasm_std::Event::new("proof_under_construction")
                .add_attribute(
                    "destination_chain",
                    serde_json::to_string(&destination_chain)
                        .expect("violated invariant: destination_chain is not serializable"),
                )
                .add_attribute(
                    "payload_id",
                    serde_json::to_string(&payload_id)
                        .expect("violated invariant: payload_id is not serializable"),
                )
                .add_attribute(
                    "multisig_session_id",
                    serde_json::to_string(&multisig_session_id)
                        .expect("violated invariant: multisig_session_id is not serializable"),
                )
                .add_attribute(
                    "message_ids",
                    serde_json::to_string(&msg_ids)
                        .expect("violated invariant: message_ids is not serializable"),
                ),
            Event::DebugEvent {
                domain_separator,
                verifier_set,
                payload,
            } => {
                let domain_separator = domain_separator
                    .iter()
                    .map(|b| format!("{}u8", b))
                    .collect::<Vec<_>>()
                    .join(", ");
                cosmwasm_std::Event::new("debug_event")
                    .add_attribute(
                        "domain_separator",
                        serde_json::to_string(&domain_separator)
                            .expect("violated invariant: domain_separator is not serializable"),
                    )
                    .add_attribute(
                        "verifier_set",
                        serde_json::to_string(&verifier_set)
                            .expect("violated invariant: verifier_set is not serializable"),
                    )
                    .add_attribute(
                        "payload",
                        serde_json::to_string(&payload)
                            .expect("violated invariant: payload is not serializable"),
                    )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use router_api::Message;
    use serde_json::to_string;

    use super::*;
    use crate::payload::Payload;

    #[test]
    fn proof_under_construction_is_serializable() {
        let payload = Payload::Messages(vec![
            Message {
                cc_id: CrossChainId::new("ethereum", "some-id").unwrap(),
                source_address: "0x1234".parse().unwrap(),
                destination_chain: "avalanche".parse().unwrap(),
                destination_address: "0x5678".parse().unwrap(),
                payload_hash: [0; 32],
            },
            Message {
                cc_id: CrossChainId::new("fantom", "some-other-id").unwrap(),
                source_address: "0x1234".parse().unwrap(),
                destination_chain: "avalanche".parse().unwrap(),
                destination_address: "0x5678".parse().unwrap(),
                payload_hash: [0; 32],
            },
        ]);

        let event = Event::ProofUnderConstruction {
            destination_chain: "avalanche".parse().unwrap(),
            payload_id: payload.id(),
            multisig_session_id: Uint64::new(2),
            msg_ids: payload.message_ids().unwrap(),
        };

        assert!(to_string(&cosmwasm_std::Event::from(event)).is_ok());
    }
}
