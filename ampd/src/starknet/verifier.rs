use axelar_wasm_std::voting::Vote;
use cosmwasm_std::HexBinary;
use multisig::key::PublicKey;
use starknet_types::events::contract_call::ContractCallEvent;
use starknet_types::events::signers_rotated::SignersRotatedEvent;

use crate::handlers::starknet_verify_msg::Message;
use crate::handlers::starknet_verify_verifier_set::VerifierSetConfirmation;

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

pub fn verify_verifier_set(
    event: &SignersRotatedEvent,
    confirmation: &VerifierSetConfirmation,
    source_gateway_address: &str,
) -> Vote {
    // nonce should never be 0
    if event.signers.nonce == [0_u8; 32] {
        return Vote::NotFound;
    }
    if event == confirmation && event.from_address == source_gateway_address {
        Vote::SucceededOnChain
    } else {
        Vote::NotFound
    }
}

impl PartialEq<VerifierSetConfirmation> for SignersRotatedEvent {
    fn eq(&self, confirmation: &VerifierSetConfirmation) -> bool {
        let expected = &confirmation.verifier_set;

        // Convert and sort expected signers
        let mut expected_signers = expected
            .signers
            .values()
            .map(|signer| (signer.pub_key.clone(), signer.weight.u128()))
            .collect::<Vec<_>>();
        expected_signers.sort();

        // Convert and sort actual signers from the event
        let mut actual_signers = self
            .signers
            .signers
            .iter()
            .map(|signer| {
                (
                    PublicKey::Ecdsa(HexBinary::from(signer.signer.as_bytes())),
                    signer.weight as u128,
                )
            })
            .collect::<Vec<_>>();
        actual_signers.sort();

        // Compare signers, threshold, and created_at timestamp
        actual_signers == expected_signers
            && self.signers.threshold == expected.threshold.u128()
            // The nonce is 32 bytes but created_at is 8 bytes (u64), so we only compare the first 8 bytes
            && self.signers.nonce[..8] == expected.created_at.to_be_bytes()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use axelar_wasm_std::msg_id::{FieldElementAndEventIndex, HexTxHashAndEventIndex};
    use axelar_wasm_std::voting::Vote;
    use cosmrs::crypto::PublicKey;
    use cosmwasm_std::{Addr, HexBinary, Uint128};
    use ecdsa::SigningKey;
    use ethers_core::types::H256;
    use multisig::key::KeyType;
    use multisig::msg::Signer;
    use multisig::verifier_set::VerifierSet;
    use rand::rngs::OsRng;
    use router_api::ChainName;
    use starknet_core::types::Felt;
    use starknet_types::events::contract_call::ContractCallEvent;
    use starknet_types::events::signers_rotated::{
        Signer as StarknetSigner, SignersRotatedEvent, WeightedSigners,
    };

    use super::verify_msg;
    use crate::handlers::starknet_verify_msg::Message;
    use crate::handlers::starknet_verify_verifier_set::VerifierSetConfirmation;
    use crate::starknet::verifier::verify_verifier_set;

    // "hello" as payload
    // "hello" as destination address
    // "some_contract_address" as source address
    // "destination_chain" as destination_chain
    fn mock_valid_event() -> ContractCallEvent {
        ContractCallEvent {
            from_contract_addr: String::from(
                "0x035410be6f4bf3f67f7c1bb4a93119d9d410b2f981bfafbf5dbbf5d37ae7439e",
            ),
            destination_address: String::from("destination_address"),
            destination_chain: String::from("ethereum"),
            source_address: Felt::from_str(
                "0x00b3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca",
            )
            .unwrap(),
            payload_hash: H256::from_slice(&[
                28, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123, 86, 217,
                81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
            ]),
        }
    }

    fn mock_valid_message() -> Message {
        Message {
            message_id: FieldElementAndEventIndex {
                tx_hash: Felt::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                )
                .unwrap(),
                event_index: 0,
            },
            destination_address: String::from("destination_address"),
            destination_chain: ChainName::from_str("ethereum").unwrap(),
            source_address: Felt::from_str(
                "0x00b3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca",
            )
            .unwrap(),
            payload_hash: H256::from_slice(&[
                28, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123, 86, 217,
                81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
            ]),
        }
    }

    #[test]
    fn shoud_fail_different_source_gw() {
        assert_eq!(
            verify_msg(
                &mock_valid_event(),
                &mock_valid_message(),
                &String::from("different"),
            ),
            Vote::NotFound
        )
    }

    #[test]
    fn shoud_fail_different_event_fields() {
        let msg = mock_valid_message();
        let source_gw_address =
            String::from("0x035410be6f4bf3f67f7c1bb4a93119d9d410b2f981bfafbf5dbbf5d37ae7439e");

        let mut event = mock_valid_event();
        event.destination_address = String::from("different");
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);

        let mut event = { mock_valid_event() };
        event.destination_chain = String::from("different");
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);

        let mut event = { mock_valid_event() };
        event.source_address = Felt::THREE;
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);

        let mut event = { mock_valid_event() };
        event.payload_hash = H256::from_slice(&[
            28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123, 86, 217, 81,
            123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234,
            1, // last byte is different
        ]);
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);
    }

    #[test]
    fn shoud_fail_different_msg_fields() {
        let event = mock_valid_event();
        let source_gw_address =
            String::from("0x035410be6f4bf3f67f7c1bb4a93119d9d410b2f981bfafbf5dbbf5d37ae7439e");

        let mut msg = mock_valid_message();
        msg.destination_address = String::from("different");
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);

        let mut msg = { mock_valid_message() };
        msg.destination_chain = ChainName::from_str("avalanche").unwrap();
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);

        let mut msg = { mock_valid_message() };
        msg.source_address = Felt::THREE;
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);

        let mut msg = { mock_valid_message() };
        msg.payload_hash = H256::from_slice(&[
            28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123, 86, 217, 81,
            123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234,
            1, // last byte is different
        ]);
        assert_eq!(verify_msg(&event, &msg, &source_gw_address), Vote::NotFound);
    }

    #[test]
    fn shoud_verify_event() {
        assert_eq!(
            verify_msg(
                &mock_valid_event(),
                &mock_valid_message(),
                &String::from("0x035410be6f4bf3f67f7c1bb4a93119d9d410b2f981bfafbf5dbbf5d37ae7439e"),
            ),
            Vote::SucceededOnChain
        )
    }

    /// Verifier set confirmation ///

    // FIXME: this is not a valid confirmation
    fn mock_valid_confirmation_signers_rotated() -> VerifierSetConfirmation {
        VerifierSetConfirmation {
            verifier_set: mock_valid_verifier_set_signers_rotated(),
            message_id: HexTxHashAndEventIndex {
                tx_hash: [0_u8; 32],
                event_index: 0,
            },
        }
    }

    // FIXME: this is not a valid verifier set
    fn mock_valid_verifier_set_signers_rotated() -> VerifierSet {
        VerifierSet::new(vec![], Uint128::one(), 1)
    }

    // FIXME: this is not a valid event
    fn mock_valid_event_signers_rotated() -> SignersRotatedEvent {
        SignersRotatedEvent {
            from_address: String::from(
                "0x0000000000000000000000000000000000000000000000000000000000000001",
            ),
            epoch: 1,
            signers_hash: [8_u8; 32],
            signers: WeightedSigners {
                signers: vec![
                    StarknetSigner {
                        signer: String::from(
                            "0x0000000000000000000000000000000000000000000000000000000000000002",
                        ),
                        weight: Uint128::one().into(),
                    },
                    StarknetSigner {
                        signer: String::from(
                            "0x0000000000000000000000000000000000000000000000000000000000000003",
                        ),
                        weight: Uint128::one().into(),
                    },
                    StarknetSigner {
                        signer: String::from(
                            "0x0000000000000000000000000000000000000000000000000000000000000004",
                        ),
                        weight: Uint128::one().into(),
                    },
                ],
                threshold: Uint128::one().into(),
                nonce: [7_u8; 32],
            },
        }
    }

    /// Creates a random signer with a randomly generated ECDSA key pair and weight of 1.
    /// Used for testing purposes.
    /// Returns a Signer struct containing the address, public key and weight.
    ///
    /// # Returns
    /// * `Signer` - A signer with random ECDSA key pair and weight of 1
    ///
    fn random_signer_cosmos() -> Signer {
        let priv_key = SigningKey::random(&mut OsRng);
        let pub_key: PublicKey = priv_key.verifying_key().into();
        let address = Addr::unchecked(pub_key.account_id("axelar").unwrap());
        let pub_key = (KeyType::Ecdsa, HexBinary::from(pub_key.to_bytes()))
            .try_into()
            .unwrap();

        Signer {
            address,
            weight: Uint128::one(),
            pub_key,
        }
    }

    #[test]
    fn should_not_verify_verifier_set_if_nonce_mismatch() {
        let mut event = mock_valid_event_signers_rotated();
        event.signers.nonce = [0_u8; 32]; // nonce should never be 0
        let gateway_address =
            String::from("0x035410be6f4bf3f67f7c1bb4a93119d9d410b2f981bfafbf5dbbf5d37ae7439e");
        let confirmation = mock_valid_confirmation_signers_rotated();

        assert_eq!(
            verify_verifier_set(&event, &confirmation, &gateway_address),
            Vote::NotFound
        );
    }
    #[test]
    fn shoud_not_verify_verifier_set_if_signers_mismatch() {
        let source_gw_address =
            String::from("0x035410be6f4bf3f67f7c1bb4a93119d9d410b2f981bfafbf5dbbf5d37ae7439e");
        let mut event = mock_valid_event_signers_rotated();
        let confirmation = mock_valid_confirmation_signers_rotated();
        event.signers.signers[0].signer =
            String::from("0x0000000000000000000000000000000000000000000000000000000000000005");

        assert_eq!(
            verify_verifier_set(&event, &confirmation, &source_gw_address),
            Vote::NotFound
        );
    }
}
