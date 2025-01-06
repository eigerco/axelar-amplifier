use std::str::FromStr as _;

use aleo_types::address::Address;
use cosmwasm_std::Uint128;
use error_stack::Report;
use multisig::key::PublicKey;
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("AleoGateway: {0}")]
    AleoGateway(String),
    #[error("Unsupported Public Key: {0}")]
    UnsupportedPublicKey(String),
    #[error("Aleo: {0}")]
    Aleo(#[from] snarkvm_wasm::program::Error),
    #[error("Hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("AleoTypes: {0}")]
    AleoTypes(#[from] aleo_types::Error),
}

#[derive(Debug, Clone)]
pub struct Hash(pub [u8; 32]);

#[derive(Debug, Clone)]
pub struct Message {
    pub cc_id: router_api::CrossChainId,
    pub source_address: String,
    pub destination_chain: router_api::ChainName,
    pub destination_address: String,
    pub payload_hash: [u8; 32], // The hash of the payload, send from the relayer of the source chain
}

#[derive(Debug, Clone)]
pub struct WeightedSigner {
    signer: Address,
    weight: u128,
}

#[derive(Debug, Clone)]
pub struct WeightedSigners {
    signers: Vec<WeightedSigner>, // TODO: [WeightedSigner; 32],
    threshold: Uint128,
    nonce: [u64; 4],
}

pub struct PayloadDigest<'a> {
    domain_separator: &'a [u8; 32],
    signers_hash: &'a [u8; 32],
    data_hash: &'a [u8; 32],
}

impl<'a> PayloadDigest<'a> {
    pub fn new(
        domain_separator: &'a [u8; 32],
        signers_hash: &'a [u8; 32],
        data_hash: &'a [u8; 32],
    ) -> PayloadDigest<'a> {
        PayloadDigest {
            domain_separator,
            signers_hash,
            data_hash,
        }
    }

    pub fn to_aleo_string(&self) -> String {
        format!(
            r#"{{ domain_separator: [ {} ], signers_hash: [ {} ], data_hash: [ {} ] }}"#,
            self.domain_separator
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", "),
            self.signers_hash
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", "),
            self.data_hash
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn hash(&self) -> Result<[u8; 32], Report<Error>> {
        hash(self.to_aleo_string())
    }
}

impl TryFrom<&VerifierSet> for WeightedSigners {
    type Error = Report<Error>;

    fn try_from(value: &VerifierSet) -> Result<Self, Self::Error> {
        let mut signers = value
            .signers
            .values()
            .map(|signer| match &signer.pub_key {
                PublicKey::AleoSchnorr(key) => Ok(WeightedSigner {
                    signer: Address::try_from(key).map_err(|e| {
                        Report::new(Error::AleoGateway(format!(
                            "Failed to parse address: {}",
                            e
                        )))
                    })?,
                    weight: signer.weight.into(),
                }),
                PublicKey::Ecdsa(_) => Err(Report::new(Error::UnsupportedPublicKey(
                    "received Ecdsa".to_string(),
                ))),
                PublicKey::Ed25519(_) => Err(Report::new(Error::UnsupportedPublicKey(
                    "received Ed25519".to_string(),
                ))),
            })
            .chain(std::iter::repeat_with(|| {
                Ok(WeightedSigner {
                    signer: Address::default(),
                    weight: Default::default(),
                })
            }))
            .take(32)
            .collect::<Result<Vec<_>, _>>()?;

        signers.sort_by(|signer1, signer2| signer1.signer.cmp(&signer2.signer));

        let threshold = value.threshold;
        let nonce = [0, 0, 0, value.created_at];

        Ok(WeightedSigners {
            signers,
            threshold,
            nonce,
        })
    }
}

impl WeightedSigners {
    pub fn to_aleo_string(&self) -> String {
        let signers = self
            .signers
            .iter()
            .map(|signer| format!("{}", signer.to_aleo_string()))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"{{ signers: [ {} ], threshold: {}u128, nonce: [ {}u64, {}u64, {}u64, {}u64 ] }}"#,
            signers, self.threshold, self.nonce[0], self.nonce[1], self.nonce[2], self.nonce[3]
        )
    }

    pub fn hash(&self) -> Result<[u8; 32], Report<Error>> {
        println!("WeightedSigners: {:?}", self.to_aleo_string());
        hash(self.to_aleo_string())
    }
}

fn hash<T: AsRef<str>>(input: T) -> Result<[u8; 32], Report<Error>> {
    let aleo_value =
        snarkvm_wasm::program::Plaintext::<snarkvm_wasm::network::TestnetV0>::from_str(
            input.as_ref(),
        )
        .map_err(|e| {
            Report::new(Error::Aleo(e))
                .attach_printable(format!("input: '{:?}'", input.as_ref().to_owned()))
        })?
        .to_bits_le();

    let bits = snarkvm_wasm::network::TestnetV0::hash_keccak256(&aleo_value).map_err(|e| {
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

impl WeightedSigner {
    pub fn to_aleo_string(&self) -> String {
        format!(
            r#"{{signer: {}, weight: {}u128}}"#,
            self.signer.0, self.weight
        )
    }

    pub fn hash(&self) -> Result<[u8; 32], Report<Error>> {
        hash(self.to_aleo_string())
    }
}

impl Message {
    pub fn aleo_string(&self) -> Result<String, Error> {
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

        let destination_address_hex: Vec<u8> = hex::decode(self.destination_address.as_str())?;
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

pub struct Messages(Vec<Message>);

impl From<Vec<Message>> for Messages {
    fn from(v: Vec<Message>) -> Self {
        Messages(v)
    }
}

use snarkvm_wasm::network::Network;
use snarkvm_wasm::program::ToBits;

impl Messages {
    pub fn to_aleo_string(&self) -> Result<String, Error> {
        let res = format!(
            r#"{{ messages: [{}] }}"#,
            self.0
                .iter()
                .map(Message::aleo_string)
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ")
        );

        Ok(res)
    }

    pub fn hash(&self) -> Result<[u8; 32], Report<Error>> {
        hash(self.to_aleo_string()?)
    }
}

// TODO: nonce is skipped

#[derive(Clone, Debug)]
pub struct RawSignature {
    pub signature: Vec<u8>,
}

impl RawSignature {
    pub fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = String::from_utf8(self.signature.clone()).map_err(|e| {
            Report::new(Error::AleoGateway(format!(
                "Failed to convert to utf8: {}",
                e
            )))
        })?;
        Ok(res)
    }
}

#[derive(Clone, Debug)]
pub struct SignerWithSignature {
    pub signer: WeightedSigner,
    pub signature: RawSignature,
}

#[derive(Clone, Debug)]
pub struct Proof {
    pub weighted_signers: WeightedSigners,
    pub signatures: Vec<RawSignature>,
}

impl Proof {
    pub fn new(
        verifier_set: VerifierSet,
        signer_with_signature: Vec<SignerWithSig>,
    ) -> Result<Self, Report<Error>> {
        let weighted_signers = WeightedSigners::try_from(&verifier_set)?;

        let mut signer_with_signature = signer_with_signature;

        signer_with_signature.sort_by(|s1, s2| s1.signer.address.cmp(&s2.signer.address));

        let signatures = signer_with_signature
            .iter()
            .cloned()
            .map(|s| {
                Ok(RawSignature {
                    signature: match s.signature {
                        multisig::key::Signature::AleoSchnorr(sig) => sig.to_vec(),
                        _ => {
                            return Err(Report::new(Error::UnsupportedPublicKey(
                                "Missing Aleo schnorr signature".to_string(),
                            )))
                        }
                    },
                })
            })
            .collect::<Result<Vec<_>, Report<Error>>>()?;

        Ok(Proof {
            weighted_signers,
            signatures,
        })
    }
}

impl Proof {
    pub fn to_aleo_string(&self) -> Result<String, Error> {
        let res = format!(
            r#"{{ weighted_signers: [{}], signatures: [{}] }}"#,
            self.weighted_signers.to_aleo_string(),
            self.signatures
                .iter()
                .map(|signature| {
                    format!(
                        r#"{}"#,
                        signature.to_aleo_string().unwrap(),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(res)
    }

    pub fn hash(&self) -> Result<[u8; 32], Report<Error>> {
        hash(self.to_aleo_string()?)
    }
}

pub struct ExecuteData {
    proof: Proof,
    payload: Messages,
}

impl ExecuteData {
    pub fn new(proof: Proof, payload: Messages) -> ExecuteData {
        ExecuteData { proof, payload }
    }

    pub fn to_aleo_string(&self) -> Result<String, Error> {
        let res = format!(
            r#"{{ proof: {}, payload: {} }}"#,
            self.proof.to_aleo_string()?,
            self.payload.to_aleo_string()?
        );

        Ok(res)
    }

    pub fn hash(&self) -> Result<[u8; 32], Report<Error>> {
        hash(self.to_aleo_string()?)
    }
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
