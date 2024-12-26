use std::str::FromStr as _;

use aleo_types::address::Address;
use cosmwasm_std::Uint128;
use error_stack::Report;
use multisig::key::PublicKey;
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
    pub payload_hash: [u8; 32],
}

pub struct WeightedSigner {
    signer: Address,
    weight: u128,
}

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
        let signers = value
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

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use cosmwasm_std::Addr;
    use cosmwasm_std::HexBinary;
    use multisig::msg::Signer;
    use router_api::CrossChainId;
    use router_api::Message as RouterMessage;

    use super::*;

    #[test]
    fn router_message_to_gateway_message() {
        let source_chain = "chain0";
        let message_id = "au14zeyyly2s2nc8f4vze5u2gs27uyjv72qds66cvre3tlwrewqdurqpsj839";
        let source_address = "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau";
        let destination_chain = "chain1";
        let payload_hash = "8c3685dc41c2eca11426f8035742fb97ea9f14931152670a5703f18fe8b392f0";
        let destination_address = "666f6f0000000000000000000000000000000000";

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
                        pub_key: PublicKey::AleoSchnorr(HexBinary::default()),
                    },
                ),
                (
                    "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n".to_string(),
                    Signer {
                        address: Addr::unchecked(
                            "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n",
                        ),
                        weight: 1u8.into(),
                        pub_key: PublicKey::AleoSchnorr(HexBinary::default()),
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
}

/*
{ messages: [{source_chain: [99u8, 104u8, 97u8, 105u8, 110u8, 48u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8], message_id: [24949u16, 12596u16, 31333u16, 31097u16, 27769u16, 12915u16, 12910u16, 25400u16, 26164u16, 30330u16, 25909u16, 30002u16, 26483u16, 12855u16, 30073u16, 27254u16, 14130u16, 29028u16, 29494u16, 13923u16, 30322u16, 25907u16, 29804u16, 30578u16, 25975u16, 29028u16, 30066u16, 29040u16, 29546u16, 14387u16, 14592u16], source_address: [82u8, 68u8, 79u8, 24u8, 53u8, 173u8, 192u8, 32u8, 134u8, 195u8, 124u8, 178u8, 38u8, 86u8, 22u8, 5u8, 226u8, 225u8, 105u8, 155u8], destination_address: [164u8, 241u8, 15u8, 118u8, 184u8, 110u8, 1u8, 185u8, 141u8, 175u8, 102u8, 163u8, 208u8, 42u8, 101u8, 225u8, 74u8, 219u8, 7u8, 103u8], payload_hash: [140u8, 54u8, 133u8, 220u8, 65u8, 194u8, 236u8, 161u8, 20u8, 38u8, 248u8, 3u8, 87u8, 66u8, 251u8, 151u8, 234u8, 159u8, 20u8, 147u8, 17u8, 82u8, 103u8, 10u8, 87u8, 3u8, 241u8, 143u8, 232u8, 179u8, 146u8, 240u8]}] }
{ signers: [ WeightedSigner {signer: aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n, weight: 1u128}, WeightedSigner {signer: aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4, weight: 1u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128}, WeightedSigner {signer: aleo1cpwac324ulhwk55wpljtq5kserrzj8dj6qw35fje7ypt2sqpv5ysj8p76w, weight: 0u128} ], threshold: 2u128, nonce: [ 0u64, 0u64, 0u64, 100u64 ] }
*/
