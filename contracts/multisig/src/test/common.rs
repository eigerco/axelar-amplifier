use axelar_wasm_std::Participant;
use cosmwasm_std::{Addr, HexBinary, Uint128, Uint64};
use k256::ecdsa::signature::hazmat::PrehashSigner;

use crate::key::{KeyType, PublicKey};
use crate::verifier_set::VerifierSet;

#[derive(Clone, Debug)]
pub struct TestSigner {
    pub address: Addr,
    pub pub_key: HexBinary,
    pub signature: HexBinary,
    pub signed_address: HexBinary,
}

pub mod ecdsa_test_data {
    use k256::ecdsa::{Signature, SigningKey};
    use router_api::cosmos_addr;
    use sha3::{Digest, Keccak256};

    use super::*;

    pub fn new(address: Addr, signing_key: SigningKey) -> TestSigner {
        let address_hash = Keccak256::digest(address.as_bytes());
        let verifying_key = signing_key.verifying_key();
        let signature: Signature = signing_key.sign_prehash(message().as_slice()).unwrap();
        let signed_address: Signature = signing_key.sign_prehash(address_hash.as_slice()).unwrap();

        TestSigner {
            address,
            pub_key: verifying_key.to_sec1_bytes().to_vec().into(),
            signature: signature.to_bytes().to_vec().into(),
            signed_address: signed_address.to_bytes().to_vec().into(),
        }
    }

    pub fn pub_key() -> HexBinary {
        HexBinary::from_hex("025e0231bfad810e5276e2cf9eb2f3f380ce0bdf6d84c3b6173499d3ddcc008856")
            .unwrap()
    }

    pub fn signature() -> HexBinary {
        HexBinary::from_hex("d7822dd89b9df02d64b91f69cff5811dfd4de16b792d9c6054b417c733bbcc542c1e504c8a1dffac94b5828a93e33a6b45d1bf59b2f9f28ffa56b8398d68a1c5")
            .unwrap()
    }

    pub fn message() -> HexBinary {
        HexBinary::from_hex("fa0609efd1dfeedfdcc8ba51520fae2d5176b7621d2560f071e801b0817e1537")
            .unwrap()
    }

    pub fn signers() -> Vec<TestSigner> {
        let addresses = vec![
            cosmos_addr!("signer1"),
            cosmos_addr!("signer2"),
            cosmos_addr!("signer3"),
        ]
        .into_iter();
        let signing_keys = vec![
            "0002735b006b54c6f73c23f3bb0331ce930baed3afe7a56629129efc54652101",
            "1f33707db21df35e138c071766c0bbdd5430869980f97ec9a90afbf0d8700d11",
            "1064549e232c591f916533b36df33e6ab9a491103912ace1e3b8b9d51b155666",
        ]
        .into_iter()
        .map(|hex| {
            k256::ecdsa::SigningKey::from_slice(HexBinary::from_hex(hex).unwrap().as_slice())
                .unwrap()
        });

        addresses
            .zip(signing_keys)
            .map(|(address, signing_key)| new(address, signing_key))
            .collect()
    }
}

pub mod ed25519_test_data {
    use k256::ecdsa::signature::SignerMut;
    use router_api::cosmos_addr;
    use sha3::{Digest, Keccak256};

    use super::*;

    pub fn new(address: Addr, mut signing_key: ed25519_dalek::SigningKey) -> TestSigner {
        let address_hash = Keccak256::digest(address.as_bytes());
        let verifying_key = signing_key.verifying_key();
        let signature = signing_key.sign(message().as_slice());
        let signed_address = signing_key.sign(address_hash.as_slice());

        TestSigner {
            address,
            pub_key: verifying_key.to_bytes().to_vec().into(),
            signature: signature.to_bytes().to_vec().into(),
            signed_address: signed_address.to_bytes().to_vec().into(),
        }
    }

    pub fn pub_key() -> HexBinary {
        HexBinary::from_hex("45e67eaf446e6c26eb3a2b55b64339ecf3a4d1d03180bee20eb5afdd23fa644f")
            .unwrap()
    }

    pub fn signature() -> HexBinary {
        HexBinary::from_hex("bfbcd8e1f5ed0045d16738bab201ea843a2dc14af85887f0d3b17441988b356261095768578381ae5e096c08239f5d42ffd860ac706b464eb96d414abab2000c")
            .unwrap()
    }

    pub fn message() -> HexBinary {
        HexBinary::from_hex("fa0609efd1dfeedfdcc8ba51520fae2d5176b7621d2560f071e801b0817e1537")
            .unwrap()
    }

    pub fn signers() -> Vec<TestSigner> {
        let addresses = vec![
            cosmos_addr!("signer1"),
            cosmos_addr!("signer2"),
            cosmos_addr!("signer3"),
        ]
        .into_iter();
        let signing_keys = vec![
            "0002735b006b54c6f73c23f3bb0331ce930baed3afe7a56629129efc54652101",
            "1f33707db21df35e138c071766c0bbdd5430869980f97ec9a90afbf0d8700d11",
            "1064549e232c591f916533b36df33e6ab9a491103912ace1e3b8b9d51b155666",
        ]
        .into_iter()
        .map(|hex| {
            ed25519_dalek::SigningKey::try_from(HexBinary::from_hex(hex).unwrap().as_slice())
                .unwrap()
        });

        addresses
            .zip(signing_keys)
            .map(|(address, signing_key)| new(address, signing_key))
            .collect()
    }
}

#[allow(clippy::arithmetic_side_effects)]
pub fn build_verifier_set(key_type: KeyType, signers: &[TestSigner]) -> VerifierSet {
    let mut total_weight = Uint128::zero();
    let participants = signers
        .iter()
        .map(|signer| {
            total_weight += Uint128::one();
            (
                Participant {
                    address: signer.address.clone(),
                    weight: Uint128::one().try_into().unwrap(),
                },
                PublicKey::try_from((key_type, signer.pub_key.clone())).unwrap(),
            )
        })
        .collect::<Vec<_>>();

    VerifierSet::new(participants, total_weight.mul_ceil((2u64, 3u64)), 0)
}

pub mod stark_test_data {
    use cosmwasm_std::testing::MockApi;
    use sha3::{Digest, Keccak256};
    use starknet_core::types::Felt;
    use starknet_crypto::{get_public_key, rfc6979_generate_k, sign};

    use super::*;

    pub fn new(address: Addr, private_key: Felt) -> TestSigner {
        let address_hash = Keccak256::digest(address.as_bytes());

        // Get public key from private key
        let public_key = get_public_key(&private_key);

        // Sign the test message (as a Felt)
        let msg_felt = Felt::from_bytes_be_slice(message().as_slice());
        let k_msg = rfc6979_generate_k(&msg_felt, &private_key, None);
        let signature = sign(&private_key, &msg_felt, &k_msg).unwrap();

        // Sign the address hash (as a Felt)
        let address_felt = Felt::from_bytes_be_slice(address_hash.as_slice());
        let k_addr = rfc6979_generate_k(&address_felt, &private_key, None);
        let signed_address = sign(&private_key, &address_felt, &k_addr).unwrap();

        // Serialize signatures as r + s + v (96 bytes total)
        let sig_bytes = [
            signature.r.to_bytes_be(),
            signature.s.to_bytes_be(),
            signature.v.to_bytes_be(),
        ]
        .concat();

        let signed_addr_bytes = [
            signed_address.r.to_bytes_be(),
            signed_address.s.to_bytes_be(),
            signed_address.v.to_bytes_be(),
        ]
        .concat();

        TestSigner {
            address,
            pub_key: public_key.to_bytes_be().into(),
            signature: sig_bytes.into(),
            signed_address: signed_addr_bytes.into(),
        }
    }

    pub fn pub_key() -> HexBinary {
        // This is the public key for the private key used to generate the signature below
        // Private key: 0x01234567890abcdef1234567890abcdef1234567890abcdef1234567890abcde
        // We'll need to compute this properly or use a known test vector
        HexBinary::from_hex("01ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca")
            .unwrap()
    }

    pub fn signature() -> HexBinary {
        HexBinary::from_hex(
            "062b585d762287a317f4397e02594a866a24ff9d332bbf1475b140b89ee5b231\
             06d7cb4d8eb9a406e5e7b4b8707b1c39bdd2517a9204f2a1def45b003dc8a87c\
             0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap()
    }

    pub fn message() -> HexBinary {
        HexBinary::from_hex("fa0609efd1dfeedfdcc8ba51520fae2d5176b7621d2560f071e801b0817e1537")
            .unwrap()
    }

    pub fn signers() -> Vec<TestSigner> {
        let api = MockApi::default();
        let addresses = vec!["signer1", "signer2", "signer3"]
            .into_iter()
            .map(|name| api.addr_make(name));

        let private_keys = vec!["0x01", "0x02", "0x03"]
            .into_iter()
            .map(|hex| Felt::from_hex(hex).unwrap());

        addresses
            .zip(private_keys)
            .map(|(address, private_key)| new(address, private_key))
            .collect()
    }
}

// Returns a list of (key_type, subkey, signers, session_id)
pub fn signature_test_data<'a>(
    ecdsa_subkey: &'a String,
    ed25519_subkey: &'a String,
    stark_subkey: &'a String,
) -> Vec<(KeyType, &'a String, Vec<TestSigner>, Uint64)> {
    vec![
        (
            KeyType::Ecdsa,
            ecdsa_subkey,
            ecdsa_test_data::signers(),
            Uint64::from(1u64),
        ),
        (
            KeyType::Ed25519,
            ed25519_subkey,
            ed25519_test_data::signers(),
            Uint64::from(2u64),
        ),
        (
            KeyType::Stark,
            stark_subkey,
            stark_test_data::signers(),
            Uint64::from(3u64),
        ),
    ]
}
