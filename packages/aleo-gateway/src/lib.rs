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
mod string_encoder;
mod weighted_signer;
mod weighted_signers;

pub use execute_data::*;
pub use message::*;
pub use messages::*;
pub use payload_digest::*;
pub use proof::*;
pub use raw_signature::*;
pub use signer_with_signature::*;
pub use string_encoder::*;
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
    #[error("InvalidSourceChainLength: expected: {expected}, actual: {actual}")]
    InvalidEncodedStringLength { expected: usize, actual: usize },
    #[error("Invalid ascii character")]
    InvalidAscii,
}

pub trait AleoValue {
    fn to_aleo_string(&self) -> Result<String, Report<Error>>;

    fn hash<N: Network>(&self) -> Result<[u8; 32], Report<Error>> {
        let input = self.to_aleo_string()?;
        hash::<std::string::String, N>(input)
    }

    fn bhp<N: Network>(&self) -> Result<String, Report<Error>> {
        let input = self.to_aleo_string()?;
        aleo_hash::<std::string::String, N>(input)
    }
}

fn aleo_hash<T: AsRef<str>, N: Network>(input: T) -> Result<String, Report<Error>> {
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

    let group = N::hash_to_group_bhp256(&bits).unwrap();

    Ok(group.to_string())
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
    // use snarkvm::prelude::PrivateKey;
    use snarkvm_cosmwasm::account::PrivateKey;
    use snarkvm_cosmwasm::network::ToFields;
    use snarkvm_cosmwasm::types::{Address, Field};
    use string_encoder::StringEncoder;

    use super::*;

    fn router_message() -> RouterMessage {
        let source_chain = "aleo-2";
        let message_id = "au1dudq5uvhtvqry59cfm5nz4pwu95hyrak427nqqk2uvg5n530hg9su50yfm";
        let source_address = "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau";
        let destination_chain = "aleo-2";
        let destination_address = "666f6f";
        // let destination_address = "666f6f0000000000000000000000000000000000";
        let payload_hash = "a432dc983dfe6fc48bb47a90915465d9c8185b1c2aea5c87f85818cba35051c6";

        RouterMessage {
            cc_id: CrossChainId::new(source_chain, message_id).unwrap(),
            source_address: source_address.parse().unwrap(),
            destination_address: destination_address.parse().unwrap(),
            destination_chain: destination_chain.parse().unwrap(),
            payload_hash: HexBinary::from_hex(payload_hash)
                .unwrap()
                .to_array::<32>()
                .unwrap(),
        }
    }

    // fn message(router_message: RouterMessage) -> Message {
    //     Message::try_from(&router_messages).unwrap()
    // }

    // fn from_aleo_string(input: String) {
    //     const PATTERN: &str = r#"\{source_chain: \[([0-9]+u128, [0-9]+u128)\], message_id: \[([0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128)\], source_address: \[([0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128)\], contract_address: \[([0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128)\], payload_hash: \[([0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8)\]\}"#;
    //
    //     let reg = regex::Regex::new(PATTERN).unwrap();
    //
    //     let (_, [source_chain, message_id, source_address, contract_address, payload_hash]) =
    //         reg.captures(input.as_str()).unwrap().extract();
    //
    //     let source_chain = StringEncoder::from_aleo_value(source_chain).unwrap();
    //     let message_id = StringEncoder::from_aleo_value(message_id).unwrap();
    //     let source_address = StringEncoder::from_aleo_value(source_address).unwrap();
    //     let contract_address = StringEncoder::from_aleo_value(contract_address).unwrap();
    //
    //     println!("source_chain: {:?}", source_chain.decode());
    //     println!("message_id: {:?}", message_id.decode());
    //     println!("source_address: {:?}", source_address.decode());
    //     println!("contract_address: {:?}", contract_address.decode());
    //     println!("payload_hash: {:?}", payload_hash);
    // }

    #[test]
    fn sanity_test_encode_decode() {
        let router_message = router_message();
        let message = Message::try_from(&router_message).unwrap();
        let aleo_string = message.to_aleo_string().unwrap();
        println!("aleo_string: {:?}", aleo_string);

        const PATTERN: &str = r#"\{source_chain: \[([0-9]+u128, [0-9]+u128)\], message_id: \[([0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128)\], source_address: \[([0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128)\], contract_address: \[([0-9]+u128, [0-9]+u128, [0-9]+u128, [0-9]+u128)\], payload_hash: \[([0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8, [0-9]+u8)\]\}"#;

        let reg = regex::Regex::new(PATTERN).unwrap();

        let (_, [source_chain, message_id, source_address, contract_address, payload_hash]) =
            reg.captures(aleo_string.as_str()).unwrap().extract();

        let source_chain = StringEncoder::from_aleo_value(source_chain)
            .unwrap()
            .decode();
        let message_id = StringEncoder::from_aleo_value(message_id).unwrap().decode();
        let source_address = StringEncoder::from_aleo_value(source_address)
            .unwrap()
            .decode();
        let contract_address = StringEncoder::from_aleo_value(contract_address)
            .unwrap()
            .decode();
        let payload_hash = "a432dc983dfe6fc48bb47a90915465d9c8185b1c2aea5c87f85818cba35051c6";

        println!("source_chain: {:?}", source_chain);
        println!("message_id: {:?}", message_id);
        println!("source_address: {:?}", source_address);
        println!("contract_address: {:?}", contract_address);

        let next_message = Message {
            cc_id: CrossChainId::new(source_chain.clone(), message_id).unwrap(),
            source_address: source_address.parse().unwrap(),
            destination_address: contract_address.parse().unwrap(),
            destination_chain: source_chain.parse().unwrap(),
            payload_hash: message.payload_hash,
        };

        assert_eq!(message, next_message);
    }

    #[test]
    fn router_message_to_gateway_message() {
        let source_chain = "aleo-2";
        let message_id = "au1dudq5uvhtvqry59cfm5nz4pwu95hyrak427nqqk2uvg5n530hg9su50yfm";
        let source_address = "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau";
        let destination_chain = "aleo-2";
        let destination_address = "666f6f0000000000000000000000000000000000";
        let payload_hash = "a432dc983dfe6fc48bb47a90915465d9c8185b1c2aea5c87f85818cba35051c6";

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

        let message = Message::try_from(&router_messages).unwrap();
        println!("messages: {:?}", message.to_aleo_string());
        // assert_eq!(message.to_aleo_string().unwrap(), "{source_chain: [97u8, 108u8, 101u8, 111u8, 45u8, 50u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8], message_id: [24949u16, 12644u16, 30052u16, 28981u16, 30070u16, 26740u16, 30321u16, 29305u16, 13625u16, 25446u16, 27957u16, 28282u16, 13424u16, 30581u16, 14645u16, 26745u16, 29281u16, 27444u16, 12855u16, 28273u16, 29035u16, 12917u16, 30311u16, 13678u16, 13619u16, 12392u16, 26425u16, 29557u16, 13616u16, 31078u16, 27904u16], source_address: [24940u16, 25967u16, 12592u16, 26221u16, 29553u16, 30568u16, 12341u16, 14709u16, 29037u16, 14132u16, 30774u16, 29750u16, 31335u16, 27193u16, 13175u16, 26232u16, 29797u16, 28720u16, 24950u16, 25974u16, 25464u16, 31280u16, 28212u16, 30521u16, 30049u16, 30585u16, 28011u16, 30265u16, 29495u16, 30568u16, 29537u16, 29952u16], contract_address: [102u8, 111u8, 111u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8], payload_hash: [164u8, 50u8, 220u8, 152u8, 61u8, 254u8, 111u8, 196u8, 139u8, 180u8, 122u8, 144u8, 145u8, 84u8, 101u8, 217u8, 200u8, 24u8, 91u8, 28u8, 42u8, 234u8, 92u8, 135u8, 248u8, 88u8, 24u8, 203u8, 163u8, 80u8, 81u8, 198u8]}");
        // assert_eq!(
        //     messages.hash().unwrap(),
        //     [
        //         214, 16, 153, 136, 99, 187, 96, 122, 5, 161, 119, 97, 3, 227, 66, 18, 220, 166,
        //         126, 242, 200, 101, 255, 21, 252, 192, 138, 54, 210, 195, 215, 116
        //     ]
        // );
    }

    // #[test]
    // fn verifier_set_to_wighted_signers() {
    //     let verifier_set = VerifierSet {
    //         signers: BTreeMap::from_iter(vec![
    //             (
    //                 "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4".to_string(),
    //                 Signer {
    //                     address: Addr::unchecked(
    //                         "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4",
    //                     ),
    //                     weight: 1u8.into(),
    //                     pub_key: PublicKey::AleoSchnorr(HexBinary::from(
    //                         "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4"
    //                             .as_bytes(),
    //                     )),
    //                 },
    //             ),
    //             (
    //                 "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n".to_string(),
    //                 Signer {
    //                     address: Addr::unchecked(
    //                         "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n",
    //                     ),
    //                     weight: 1u8.into(),
    //                     pub_key: PublicKey::AleoSchnorr(HexBinary::from(
    //                         "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n"
    //                             .as_bytes(),
    //                     )),
    //                 },
    //             ),
    //         ]),
    //         threshold: 2u8.into(),
    //         created_at: 100u64,
    //     };
    //
    //     let weighted_signers = WeightedSigners::try_from(&verifier_set).unwrap();
    //
    //     println!("weighted_signers: {:?}", weighted_signers.to_aleo_string());
    //     println!("hash: {:?}", weighted_signers.hash().unwrap());
    // }
    //
    // fn message() -> Message {
    //     let source_chain = "chain0";
    //     let message_id = "au14zeyyly2s2nc8f4vze5u2gs27uyjv72qds66cvre3tlwrewqdurqpsj839";
    //     let source_address = "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau";
    //     let destination_chain = "chain1";
    //     let destination_address = "666f6f0000000000000000000000000000000000";
    //     let payload_hash = "8c3685dc41c2eca11426f8035742fb97ea9f14931152670a5703f18fe8b392f0";
    //
    //     let router_messages = RouterMessage {
    //         cc_id: CrossChainId::new(source_chain, message_id).unwrap(),
    //         source_address: source_address.parse().unwrap(),
    //         destination_address: destination_address.parse().unwrap(),
    //         destination_chain: destination_chain.parse().unwrap(),
    //         payload_hash: HexBinary::from_hex(payload_hash)
    //             .unwrap()
    //             .to_array::<32>()
    //             .unwrap(),
    //     };
    //     Message::try_from(&router_messages).unwrap()
    // }
    //
    // fn weighted_signers() -> WeightedSigners {
    //     let verifier_set = VerifierSet {
    //         signers: BTreeMap::from_iter(vec![
    //             (
    //                 "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4".to_string(),
    //                 Signer {
    //                     address: Addr::unchecked(
    //                         "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4",
    //                     ),
    //                     weight: 1u8.into(),
    //                     pub_key: PublicKey::AleoSchnorr(HexBinary::from(
    //                         "aleo1xpc0kpexvqc29eskjfkuyrervtqr8a8tptnmp7rhdg964xlw55psq5dnk4"
    //                             .as_bytes(),
    //                     )),
    //                 },
    //             ),
    //             (
    //                 "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n".to_string(),
    //                 Signer {
    //                     address: Addr::unchecked(
    //                         "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n",
    //                     ),
    //                     weight: 1u8.into(),
    //                     pub_key: PublicKey::AleoSchnorr(HexBinary::from(
    //                         "aleo1p8utxn802p9wrextsfjynz04rg6eq36404jxvt40sy89jpfl0qzq20l35n"
    //                             .as_bytes(),
    //                     )),
    //                 },
    //             ),
    //         ]),
    //         threshold: 2u8.into(),
    //         created_at: 100u64,
    //     };
    //
    //     WeightedSigners::try_from(&verifier_set).unwrap()
    // }
    //
    // #[test]
    // fn payload_digest() {
    //     let messages = Messages::from(vec![message()]);
    //     let weighted_signers = weighted_signers();
    //
    //     let messages_hash = messages.hash().unwrap();
    //     let weighted_signers_hash = weighted_signers.hash().unwrap();
    //     let domain_separator: [u8; 32] =
    //         hex::decode("6973c72935604464b28827141b0a463af8e3487616de69c5aa0c785392c9fb9f")
    //             .unwrap()
    //             .try_into()
    //             .unwrap();
    //
    //     let payload_digest =
    //         PayloadDigest::new(&domain_separator, &weighted_signers_hash, &messages_hash);
    //
    //     let hash = hash(payload_digest.to_aleo_string()).unwrap();
    //     assert_eq!(
    //         hash,
    //         [
    //             43u8, 248u8, 246u8, 8u8, 215u8, 130u8, 185u8, 164u8, 19u8, 56u8, 205u8, 182u8,
    //             249u8, 113u8, 213u8, 193u8, 20u8, 116u8, 38u8, 252u8, 144u8, 133u8, 179u8, 34u8,
    //             30u8, 137u8, 68u8, 203u8, 100u8, 157u8, 71u8, 143u8
    //         ]
    //     );
    // }

    use multisig::key::PublicKey;
    use multisig::msg::SignerWithSig;
    use multisig::verifier_set::{self, VerifierSet};
    // use snarkvm_::prelude::{Field, Network, PrivateKey, Signature, ToFields};

    fn aleo_encoded<N: Network>(data: &[u8; 32]) -> Vec<Field<N>> {
        let message = [
            "[",
            data.as_ref()
                .iter()
                .map(|b| format!("{:?}u8", b))
                .collect::<Vec<_>>()
                .join(", ")
                .as_str(),
            "]",
        ]
        .concat();

        snarkvm_cosmwasm::program::Value::from_str(message.as_str())
            .unwrap()
            .to_fields()
            .unwrap()
    }

    pub fn payload_digest<N: Network>(verifier_set: &VerifierSet) -> [u8; 32] {
        let message = router_message();
        let message = Message::try_from(&message).unwrap();
        let data_hash = message.hash::<N>().unwrap();
        println!("data_hash: {:?}", data_hash);

        // let addr2 = "aleo17ge8zcz2js90h8mvgpr4w5wqqy56jlt2txznh4zavvvrtrzra5qs7v0n84";

        // 6973c72935604464b28827141b0a463af8e3487616de69c5aa0c785392c9fb9f
        let domain_separator =
            hex::decode("6973c72935604464b28827141b0a463af8e3487616de69c5aa0c785392c9fb9f")
                .unwrap();
        let part1 = u128::from_le_bytes(domain_separator[0..16].try_into().unwrap());
        let part2 = u128::from_le_bytes(domain_separator[16..32].try_into().unwrap());
        let domain_separator: [u128; 2] = [part1, part2];
        println!("domain_separator: {:2x?}", domain_separator);

        let payload_digest =
            PayloadDigest::new(&domain_separator, verifier_set, &data_hash).unwrap();

        payload_digest.hash::<N>().unwrap()
    }

    #[test]
    fn proof() {
        // APrivateKey1zkp2BXKmUopUZoU7b2necZ4xSxJnBazytJoSTXiPDqvqUny
        // AViewKey1dYXjvYok2j5543n2Bkdy5mQqhtsKWv5QorWmCyJp3rMu
        // let addr1 = "aleo1hhssuewasdcya3xjepae5eum0dqja0m4z6carnr5qfp6lnqtxyqskavctq";

        //
        // APrivateKey1zkpB9x9aKfPwHBALJFCpENtteCkBkFPCCSmgyVWeqZ6PgKy
        // AViewKey1hoTRJKFTK3VUiKDFBVCfFBqH2atcT4eTFmNYysNe9xch
        // let addr2 = "aleo17ge8zcz2js90h8mvgpr4w5wqqy56jlt2txznh4zavvvrtrzra5qs7v0n84";

        // let verifier_set = VerifierSet {
        //     signers: BTreeMap::from_iter(vec![
        //         (
        //             addr1.to_string(),
        //             Signer {
        //                 address: Addr::unchecked(addr1),
        //                 weight: 1u8.into(),
        //                 pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr1.as_bytes())),
        //             },
        //         ),
        //         (
        //             addr2.to_string(),
        //             Signer {
        //                 address: Addr::unchecked(addr2),
        //                 weight: 1u8.into(),
        //                 pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr2.as_bytes())),
        //             },
        //         ),
        //     ]),
        //     threshold: 2u8.into(),
        //     created_at: 100u64,
        // };

        let private_key: PrivateKey<snarkvm_cosmwasm::network::TestnetV0> =
            PrivateKey::new(&mut rand::thread_rng()).unwrap();
        let addr = Address::try_from(private_key).unwrap();
        let addr = addr.to_string();

        let verifier_set = VerifierSet {
            signers: BTreeMap::from_iter(vec![(
                addr.to_string(),
                Signer {
                    address: Addr::unchecked(addr.as_str()),
                    weight: 1u8.into(),
                    pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr.as_str().as_bytes())),
                },
            )]),
            threshold: 2u8.into(),
            created_at: 100u64,
        };

        let payload_digest = payload_digest::<snarkvm_cosmwasm::network::TestnetV0>(&verifier_set);
        println!("payload_digest: {:?}", payload_digest);
        let aleo_value = aleo_encoded(&payload_digest);

        let signature = snarkvm_cosmwasm::program::Signature::sign(
            &private_key,
            &aleo_value,
            &mut rand::thread_rng(),
        )
        .unwrap();

        let sign = signature.to_string();

        //foo
        // let sign1 = "sign1enc46v36a9kdc7wenwmd8pm27x9hphvqw4f05suqjuxky2rjdqqvqvl3w09u7sg99ky7kcelprw7ww2aglmunfjhmatzm5hx0fhdyqzqutkjzvshmqjnxvfdu7qnkrplyesp88uwarr38z44mwfe998aqnr6tvq6qwastx9942fphg99l522t89kvrswfdea5nxttgsxwqwsx7wdty9";
        // bar
        // let sign2 = "sign1zq7azu97z7rdkchqe9nlun7t5sjnyhlrxks93fyu3f9yfu68qcpr4zlwvqtka36lcaq06yxf3u6h2jkytc5ajdh0t0xr5vjtfjrvqqzkwlnfxjjxg6gr0lzlxth7rq6xq27828zs6s75tlskrwvw2hfhpks52z8m5un66eqt0yp8ehe6498ljtd4skqyu8ftfesvh28qypeqjpxrglp";

        let signer_with_sig = SignerWithSig {
            signer: Signer {
                address: Addr::unchecked(addr.to_string()),
                weight: 1u8.into(),
                pub_key: PublicKey::AleoSchnorr(HexBinary::from(addr.to_string().as_bytes())),
            },
            signature: multisig::key::Signature::AleoSchnorr(HexBinary::from(sign.as_bytes())),
        };

        let proof = Proof::new(verifier_set, signer_with_sig).unwrap();
        let aleo_string = proof.to_aleo_string().unwrap();
        println!("proof: {:?}", aleo_string);

        let router_message = router_message();
        let m = Message::try_from(&router_message).unwrap();
        let execute_data = ExecuteData::new(proof, m);
        let res = execute_data.to_aleo_string().unwrap();
        println!("execute_data: {:?}", res);
    }
}
