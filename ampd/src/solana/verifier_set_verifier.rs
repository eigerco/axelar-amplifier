use axelar_wasm_std::voting::Vote;
use multisig::key::PublicKey;
use multisig::verifier_set::VerifierSet;
use sha3::Digest;
use sha3::Keccak256;

use crate::handlers::solana_verify_verifier_set::VerifierSetConfirmation;
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedConfirmedTransactionWithStatusMeta,
};
use thiserror::Error;
use tracing::error;

use gmp_gateway::events::{EventContainer, GatewayEvent};

#[derive(Error, Debug, PartialEq)]
pub enum VerificationError {
    #[error("Failed to parse tx log messages")]
    NoLogMessages,
    #[error("Tried to get gateway event from program logs, but couldn't find anything.")]
    NoGatewayEventFound,
}

type Result<T> = std::result::Result<T, VerificationError>;

pub fn parse_gateway_event(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<EventContainer> {
    let Some(meta) = &tx.transaction.meta else {
        return Err(VerificationError::NoLogMessages);
    };

    let OptionSerializer::Some(log_messages) = &meta.log_messages else {
        return Err(VerificationError::NoLogMessages);
    };

    log_messages
        .iter()
        .find_map(GatewayEvent::parse_log)
        .ok_or(VerificationError::NoGatewayEventFound)
}

#[tracing::instrument(name = "solana_verify_verifier_set")]
pub fn verify_verifier_set(
    verifier_set_conf: &VerifierSetConfirmation,
    new_signers_hash: &[u8; 32],
) -> Vote {
    let axelar_verifier_set_hash = hash_verifier_set(&verifier_set_conf.verifier_set);
    if &axelar_verifier_set_hash == new_signers_hash {
        return Vote::SucceededOnChain;
    }
    Vote::FailedOnChain
}

fn hash_verifier_set(verifier_set: &VerifierSet) -> [u8; 32] {
    let mut hasher = Keccak256::new();

    // Length prefix the bytes to be hashed to prevent hash collisions
    let len = u32::try_from(verifier_set.signers.len())
        .expect("impossible for the value to be larger than u32 on wasm32");
    hasher.update(len.to_le_bytes());

    verifier_set.signers.values().for_each(|signer| {
        match signer.pub_key {
            PublicKey::Ecdsa(_) => hasher.update(b"secp256k1"),
            PublicKey::Ed25519(_) => hasher.update(b"ed25519"),
        }
        hasher.update(signer.pub_key.as_ref());
        hasher.update(signer.weight.to_le_bytes());
    });

    hasher.update(verifier_set.threshold.to_le_bytes());
    hasher.update(verifier_set.created_at.to_le_bytes());

    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use axelar_rkyv_encoding::hasher::generic::Keccak256Hasher;
    use cosmwasm_std::{Addr, HexBinary};
    use multisig::{
        key::KeyType,
        test::common::{build_verifier_set, TestSigner},
    };

    #[test]
    fn test_axelar_verifier_set_hashing_is_eq_to_rkyv_hashing() {
        let (verifier_set_conf, sol_verifier_set) = matching_verifier_set_and_sol_data();

        let vote = verify_verifier_set(
            &verifier_set_conf,
            &sol_verifier_set.hash(Keccak256Hasher::default()),
        );

        assert_eq!(Vote::SucceededOnChain, vote);
    }

    fn matching_verifier_set_and_sol_data() -> (
        VerifierSetConfirmation,
        axelar_rkyv_encoding::types::VerifierSet,
    ) {
        let verifier_set = build_verifier_set(KeyType::Ecdsa, &signers());

        let sol_signers = verifier_set
            .signers
            .values()
            .map(|v| {
                let pair = (
                    axelar_rkyv_encoding::types::PublicKey::from_str(v.address.as_str()).unwrap(),
                    1.into(),
                );
                pair
            })
            .collect();

        let sol_quorum = verifier_set.threshold.u128();

        let sol_verifier_set =
            axelar_rkyv_encoding::types::VerifierSet::new(0, sol_signers, sol_quorum.into());

        let verifier_set_confirmation = VerifierSetConfirmation {
            tx_id: String::from("90af"),
            event_index: 1,
            verifier_set,
        };

        (verifier_set_confirmation, sol_verifier_set)
    }

    fn signers() -> Vec<TestSigner> {
        // This data is the same as ecdsa_test_data::signers() , but we are replacing the address with the
        // same value of the public key.
        vec![
            TestSigner {
                address: Addr::unchecked("025e0231bfad810e5276e2cf9eb2f3f380ce0bdf6d84c3b6173499d3ddcc008856"),
                pub_key: HexBinary::from_hex("025e0231bfad810e5276e2cf9eb2f3f380ce0bdf6d84c3b6173499d3ddcc008856")
            .unwrap(),
                signature: HexBinary::from_hex("d7822dd89b9df02d64b91f69cff5811dfd4de16b792d9c6054b417c733bbcc542c1e504c8a1dffac94b5828a93e33a6b45d1bf59b2f9f28ffa56b8398d68a1c5")
            .unwrap(),
                signed_address: HexBinary::from_hex(
                    "d9e1eb2b47cb8b7c1c2a5a32f6fa6c57d0e6fdd53eaa8c76fe7f0b3b390cfb3c40f258e476f2ca0e6a7ca2622ea23afe7bd1f873448e01eed86cd6446a403f36",
                )
                .unwrap(),
            },
            TestSigner {
                address: Addr::unchecked("036ff6f4b2bc5e08aba924bd8fd986608f3685ca651a015b3d9d6a656de14769fe"),
                pub_key: HexBinary::from_hex("036ff6f4b2bc5e08aba924bd8fd986608f3685ca651a015b3d9d6a656de14769fe")
            .unwrap(),
                signature: HexBinary::from_hex("a7ec5d1c15e84ba4b5da23fee49d77c5c81b3b1859411d1ef8193bf5a39783c76813e4cf4e1e1bfa0ea19c9f5b61d25ce978da137f3adb1730cba3d842702e72")
            .unwrap(),
                signed_address: HexBinary::from_hex(
                    "008ca739eaddd22856c30690bf9a85f16ea77784494ad01111fded80327c57c84e021608cd890341883de1ac0fcf31330243b91b22c4751542ac47115f2f4e2c",
                )
                .unwrap(),
            },
            TestSigner {
                address: Addr::unchecked("03686cbbef9f9e9a5c852883cb2637b55fc76bee6ee6a3ff636e7bea2e41beece4"),
                pub_key: HexBinary::from_hex("03686cbbef9f9e9a5c852883cb2637b55fc76bee6ee6a3ff636e7bea2e41beece4")
            .unwrap(),
                signature: HexBinary::from_hex("d1bc22fd89d97dfe4091c73d2002823ca9ab29b742ae531d2560bf2abafb313f7d2c3263d09d9aa72f01ed1d49046e39f6513ea61241fd59cc53d02fc4222351")
            .unwrap(),
                signed_address: HexBinary::from_hex(
                    "1df5a371c27772874b706dbbb41e0bc67f688b301d3c2d269e45c43389fa43b6328c32686f42242b0cdb05b3b955ce3106393d6e509bf0373340482182c865cc",
                )
                .unwrap(),
            },
        ]
    }
}