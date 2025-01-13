use std::str::FromStr as _;

use error_stack::Report;
use snarkvm_cosmwasm::network::Network;
use snarkvm_cosmwasm::program::ToBits;
use thiserror::Error;

mod execute_data;
mod message;
mod messages;
mod payload_digest;
mod proof;
mod raw_signature;
mod signer_with_signature;
mod weighted_signer;
mod weighted_signers;

pub use execute_data::*;
pub use message::*;
pub use messages::*;
pub use payload_digest::*;
pub use proof::*;
pub use raw_signature::*;
pub use signer_with_signature::*;
pub use weighted_signer::*;
pub use weighted_signers::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("AleoGateway: {0}")]
    AleoGateway(String),
    #[error("Unsupported Public Key: {0}")]
    UnsupportedPublicKey(String),
    #[error("Aleo: {0}")]
    Aleo(#[from] snarkvm_cosmwasm::program::Error),
    #[error("Hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("AleoTypes: {0}")]
    AleoTypes(#[from] aleo_types::Error),
}

pub trait AleoValue {
    fn to_aleo_string(&self) -> Result<String, Report<Error>>;

    fn hash<N: Network>(&self) -> Result<[u8; 32], Report<Error>> {
        let input = self.to_aleo_string()?;
        hash::<std::string::String, N>(input)
    }
}

fn hash<T: AsRef<str>, N: Network>(input: T) -> Result<[u8; 32], Report<Error>> {
    let aleo_value: Vec<bool> = snarkvm_cosmwasm::program::Value::<N>::from_str(input.as_ref())
        .map_err(|e| {
            Report::new(Error::Aleo(e))
                .attach_printable(format!("input: '{:?}'", input.as_ref().to_owned()))
        })?
        .to_bits_le();

    let bits = N::hash_keccak256(&aleo_value).map_err(|e| {
        Report::new(Error::Aleo(e))
            .attach_printable(format!("input2: '{:?}'", input.as_ref().to_owned()))
    })?;

    let mut hash = [0u8; 32];
    for (i, b) in bits.chunks(8).enumerate() {
        let mut byte = 0u8;
        for (i, bit) in b.iter().enumerate() {
            if *bit {
                byte |= 1 << i;
            }
        }
        hash[i] = byte;
    }

    Ok(hash)
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use cosmwasm_std::{Addr, HexBinary};
    use multisig::msg::Signer;
    use router_api::{CrossChainId, Message as RouterMessage};

    use super::*;

    #[test]
    fn router_message_to_gateway_message() {
        let source_chain = "chain0";
        let message_id = "au14zeyyly2s2nc8f4vze5u2gs27uyjv72qds66cvre3tlwrewqdurqpsj839";
        let source_address = "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau";
        let destination_chain = "chain1";
        let destination_address = "666f6f0000000000000000000000000000000000";
        let payload_hash = "8c3685dc41c2eca11426f8035742fb97ea9f14931152670a5703f18fe8b392f0";

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

        let messages = Messages::from(vec![Message::try_from(&router_messages).unwrap()]);
        println!("messages: {:?}", messages.to_aleo_string());
        // assert_eq!(
        //     messages.hash().unwrap(),
        //     [
        //         214, 16, 153, 136, 99, 187, 96, 122, 5, 161, 119, 97, 3, 227, 66, 18, 220, 166,
        //         126, 242, 200, 101, 255, 21, 252, 192, 138, 54, 210, 195, 215, 116
        //     ]
        // );
    }

    #[test]
    fn verifier_set_to_wighted_signers() {
        let verifier_set = VerifierSet {
            signers: BTreeMap::from_iter(vec![
                (
                    "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4".to_string(),
                    Signer {
                        address: Addr::unchecked(
                            "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4",
                        ),
                        weight: 1u8.into(),
                        pub_key: PublicKey::AleoSchnorr(HexBinary::from(
                            "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4"
                                .as_bytes(),
                        )),
                    },
                ),
                (
                    "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n".to_string(),
                    Signer {
                        address: Addr::unchecked(
                            "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n",
                        ),
                        weight: 1u8.into(),
                        pub_key: PublicKey::AleoSchnorr(HexBinary::from(
                            "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n"
                                .as_bytes(),
                        )),
                    },
                ),
            ]),
            threshold: 2u8.into(),
            created_at: 100u64,
        };

        let weighted_signers = WeightedSigners::try_from(&verifier_set).unwrap();

        println!("weighted_signers: {:?}", weighted_signers.to_aleo_string());
        println!("hash: {:?}", weighted_signers.hash().unwrap());
    }

    fn message() -> Message {
        let source_chain = "chain0";
        let message_id = "au14zeyyly2s2nc8f4vze5u2gs27uyjv72qds66cvre3tlwrewqdurqpsj839";
        let source_address = "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau";
        let destination_chain = "chain1";
        let destination_address = "666f6f0000000000000000000000000000000000";
        let payload_hash = "8c3685dc41c2eca11426f8035742fb97ea9f14931152670a5703f18fe8b392f0";

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
        Message::try_from(&router_messages).unwrap()
    }

    fn weighted_signers() -> WeightedSigners {
        let verifier_set = VerifierSet {
            signers: BTreeMap::from_iter(vec![
                (
                    "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4".to_string(),
                    Signer {
                        address: Addr::unchecked(
                            "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4",
                        ),
                        weight: 1u8.into(),
                        pub_key: PublicKey::AleoSchnorr(HexBinary::from(
                            "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4"
                                .as_bytes(),
                        )),
                    },
                ),
                (
                    "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n".to_string(),
                    Signer {
                        address: Addr::unchecked(
                            "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n",
                        ),
                        weight: 1u8.into(),
                        pub_key: PublicKey::AleoSchnorr(HexBinary::from(
                            "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n"
                                .as_bytes(),
                        )),
                    },
                ),
            ]),
            threshold: 2u8.into(),
            created_at: 100u64,
        };

        WeightedSigners::try_from(&verifier_set).unwrap()
    }

    #[test]
    fn foo() {
        let w = weighted_signers();
        println!("w: {:?}", w.to_aleo_string());
    }

    #[test]
    fn payload_digest() {
        let messages = Messages::from(vec![message()]);
        let weighted_signers = weighted_signers();

        let messages_hash = messages.hash().unwrap();
        let weighted_signers_hash = weighted_signers.hash().unwrap();
        let domain_separator: [u8; 32] =
            hex::decode("6973c72935604464b28827141b0a463af8e3487616de69c5aa0c785392c9fb9f")
                .unwrap()
                .try_into()
                .unwrap();

        let payload_digest =
            PayloadDigest::new(&domain_separator, &weighted_signers_hash, &messages_hash);

        let hash = hash(payload_digest.to_aleo_string()).unwrap();
        assert_eq!(
            hash,
            [
                43u8, 248u8, 246u8, 8u8, 215u8, 130u8, 185u8, 164u8, 19u8, 56u8, 205u8, 182u8,
                249u8, 113u8, 213u8, 193u8, 20u8, 116u8, 38u8, 252u8, 144u8, 133u8, 179u8, 34u8,
                30u8, 137u8, 68u8, 203u8, 100u8, 157u8, 71u8, 143u8
            ]
        );
    }

    #[test]
    fn proof() {
        // APrivateKey1zkp2BXKmUopUZoU7b2necZ4xSxJnBazytJoSTXiPDqvqUny
        // AViewKey1dYXjvYok2j5543n2Bkdy5mQqhtsKWv5QorWmCyJp3rMu
        let addr1 = "aleo1hhssuewasdcya3xjepae5eum0dqja0m4z6carnr5qfp6lnqtxyqskavctq";

        //
        // APrivateKey1zkpB9x9aKfPwHBALJFCpENtteCkBkFPCCSmgyVWeqZ6PgKy
        // AViewKey1hoTRJKFTK3VUiKDFBVCfFBqH2atcT4eTFmNYysNe9xch
        let addr2 = "aleo17ge8zcz2js90h8mvgpr4w5wqqy56jlt2txznh4zavvvrtrzra5qs7v0n84";

        let verifier_set = VerifierSet {
            signers: BTreeMap::from_iter(vec![
                (
                    addr1.to_string(),
                    Signer {
                        address: Addr::unchecked(addr1),
                        weight: 1u8.into(),
                        pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr1.as_bytes())),
                    },
                ),
                (
                    addr2.to_string(),
                    Signer {
                        address: Addr::unchecked(addr2),
                        weight: 1u8.into(),
                        pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr2.as_bytes())),
                    },
                ),
            ]),
            threshold: 2u8.into(),
            created_at: 100u64,
        };

        //foo
        let sign1 = "sign1enc46v36a9kdc7wenwmd8pm27x9hphvqw4f05suqjuxky2rjdqqvqvl3w09u7sg99ky7kcelprw7ww2aglmunfjhmatzm5hx0fhdyqzqutkjzvshmqjnxvfdu7qnkrplyesp88uwarr38z44mwfe998aqnr6tvq6qwastx9942fphg99l522t89kvrswfdea5nxttgsxwqwsx7wdty9";
        // bar
        let sign2 = "sign1zq7azu97z7rdkchqe9nlun7t5sjnyhlrxks93fyu3f9yfu68qcpr4zlwvqtka36lcaq06yxf3u6h2jkytc5ajdh0t0xr5vjtfjrvqqzkwlnfxjjxg6gr0lzlxth7rq6xq27828zs6s75tlskrwvw2hfhpks52z8m5un66eqt0yp8ehe6498ljtd4skqyu8ftfesvh28qypeqjpxrglp";

        let signer_with_sig = vec![
            SignerWithSig {
                signer: Signer {
                    address: Addr::unchecked(addr1),
                    weight: 1u8.into(),
                    pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr1.as_bytes())),
                },
                signature: multisig::key::Signature::AleoSchnorr(HexBinary::from(sign1.as_bytes())),
            },
            SignerWithSig {
                signer: Signer {
                    address: Addr::unchecked(addr2),
                    weight: 1u8.into(),
                    pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr2.as_bytes())),
                },
                signature: multisig::key::Signature::AleoSchnorr(HexBinary::from(sign2.as_bytes())),
            },
        ];

        let proof = Proof::new(verifier_set, signer_with_sig).unwrap();
        let aleo_string = proof.to_aleo_string().unwrap();
        println!("proof: {:?}", aleo_string);
    }
}
