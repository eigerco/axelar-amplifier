//! Augments regular json parsing by recognizing and parsing aleo-json strings

use std::fmt;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

use super::AleoJson;

struct AleoJsonVisitor;

impl Visitor<'_> for AleoJsonVisitor {
    type Value = AleoJson;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing aleo json data")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let Ok(json) = crate::aleo_json::parse::into_json(v) else {
            return Ok(AleoJson::NotYetParsable(v.to_string()));
        };

        // new variants must be manually added here, compiler can't help
        if let Ok(pay_for_gas_contract_call) = serde_json::from_str(&json) {
            Ok(AleoJson::GasPaidForContractCall(pay_for_gas_contract_call))
        } else if let Ok(add_gas) = serde_json::from_str(&json) {
            Ok(AleoJson::GasAddedOrRefunded(add_gas))
        } else if let Ok(call_contract) = serde_json::from_str(&json) {
            Ok(AleoJson::CallContract(call_contract))
        } else if let Ok(remote_deploy_interchain_token) = serde_json::from_str(&json) {
            Ok(AleoJson::RemoteDeployInterchainToken(
                remote_deploy_interchain_token,
            ))
        } else {
            Ok(AleoJson::Untargeted(v.to_string()))
        }
    }
}

impl<'de> Deserialize<'de> for AleoJson {
    fn deserialize<D>(deserializer: D) -> Result<AleoJson, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AleoJsonVisitor)
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;

    use crate::aleo_json::AleoJson;

    #[test]
    fn test_aleo_json_deserialization() {
        let json_containing_aleo_json = r#"
            {
                "type": "public",
                "id": "5941340360366012957022186983990627289197739244439700389660452408222320542225field",
                "value": "{\n  caller: aleo1rtxa7fxfsznuulgcfc77prmwvw7g4y2y7r7xl4xltcygjpn34yzsh2dmln,\n  signer: aleo1rtxa7fxfsznuulgcfc77prmwvw7g4y2y7r7xl4xltcygjpn34yzsh2dmln,\n  gas: 42u64,\n  destination_chain: 0u16,\n  destination_address: [\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8,\n    0u8\n  ],\n  payload_hash: 8429161448038149565051580476768369125200938243050193788882591192533735538094field,\n  execution_gas_limit: 0u64,\n  refund_address: aleo1rtxa7fxfsznuulgcfc77prmwvw7g4y2y7r7xl4xltcygjpn34yzsh2dmln\n}"
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Output {
            id: String,
            value: AleoJson,
        }

        let output: Output = serde_json::from_str(json_containing_aleo_json).unwrap();
        assert_eq!(
            output.id,
            "5941340360366012957022186983990627289197739244439700389660452408222320542225field"
        );

        let AleoJson::GasPaidForContractCall(gas_paid_for_contract_call) = output.value else {
            panic!("Expected GasPaidForContractCall variant");
        };

        assert_eq!(gas_paid_for_contract_call.gas, 42);
        assert_eq!(
            gas_paid_for_contract_call.refund_address,
            "aleo1rtxa7fxfsznuulgcfc77prmwvw7g4y2y7r7xl4xltcygjpn34yzsh2dmln"
        );
    }

    #[test]
    fn call_contract_aleo_json_deserialization() {
        let call_contract_aleo_json = r#"
            {
                "type": "public",
                "id": "1838630682639301546698476815593374400624705850793286684052515692838117583551field",
                "value": "{\n  caller: aleo1ymrcwun5g9z0un8dqgdln7l3q77asqr98p7wh03dwgk4yfltpqgq9efvfz,\n  sender: aleo1s3ws5tra87fjycnjrwsjcrnw2qxr8jfqqdugnf0xzqqw29q9m5pqem2u4t,\n  destination_chain: [\n    129560248324330402842460762574046625792u128,\n    0u128\n  ],\n  destination_address: [\n    129560248324330635220088419148146701675u128,\n    146767682061739132652181577970743343734u128,\n    67091725296194228626838386705843189365u128,\n    141160062220609416535136629668810482795u128,\n    69119855780815625390997967134577917952u128,\n    0u128\n  ]\n}"
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Output {
            id: String,
            value: AleoJson,
        }

        let output: Output = serde_json::from_str(call_contract_aleo_json).unwrap();
        assert_eq!(
            output.id,
            "1838630682639301546698476815593374400624705850793286684052515692838117583551field"
        );

        let AleoJson::CallContract(call_contract) = output.value else {
            panic!("Expected CallContract variant");
        };

        assert_eq!(
            call_contract.caller,
            "aleo1ymrcwun5g9z0un8dqgdln7l3q77asqr98p7wh03dwgk4yfltpqgq9efvfz"
        );
        assert_eq!(
            call_contract.sender,
            "aleo1s3ws5tra87fjycnjrwsjcrnw2qxr8jfqqdugnf0xzqqw29q9m5pqem2u4t"
        );
        assert_eq!(
            call_contract.destination_chain,
            [129560248324330402842460762574046625792u128, 0u128]
        );
        assert_eq!(
            call_contract.destination_address,
            [
                129560248324330635220088419148146701675u128,
                146767682061739132652181577970743343734u128,
                67091725296194228626838386705843189365u128,
                141160062220609416535136629668810482795u128,
                69119855780815625390997967134577917952u128,
                0u128
            ]
        );
    }

    #[test]
    fn its_remote_deploy_interchain_token_payload() {
        let call_contract_aleo_json = r#"
            {
                "type": "public",
                "id": "4984440401782481626869257166278536490033759796081903177303506920110532684796field",
                "value": "{\n  info: {\n    name: 112233674851240411241919795483029536768u128,\n    symbol: 112086112285191324017087824258874212352u128,\n    decimals: 10u8\n  },\n  token_id: [\n    0u128,\n    1u128\n  ],\n  destination_chain: [\n    0u128,\n    0u128\n  ],\n  has_minter: false,\n  minter: [\n    0u128,\n    0u128,\n    0u128,\n    0u128,\n    0u128,\n    0u128\n  ]\n}"
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Output {
            id: String,
            value: AleoJson,
        }

        let output: Output = serde_json::from_str(call_contract_aleo_json).unwrap();
        assert_eq!(
            output.id,
            "4984440401782481626869257166278536490033759796081903177303506920110532684796field"
        );

        println!("Deserialized output: {:?}", output);
        let AleoJson::RemoteDeployInterchainToken(remote_deploy_interchain_token) = output.value
        else {
            panic!("Expected RemoteDeployInterchainToken variant");
        };

        assert_eq!(
            remote_deploy_interchain_token.info.name,
            112233674851240411241919795483029536768u128
        );
        assert_eq!(
            remote_deploy_interchain_token.info.symbol,
            112086112285191324017087824258874212352u128
        );
        assert_eq!(remote_deploy_interchain_token.info.decimals, 10u8);
        assert_eq!(remote_deploy_interchain_token.token_id, [0u128, 1u128]);
        assert_eq!(
            remote_deploy_interchain_token.destination_chain,
            [0u128, 0u128]
        );
        assert!(!remote_deploy_interchain_token.has_minter);
        assert_eq!(
            remote_deploy_interchain_token.minter,
            [0u128, 0u128, 0u128, 0u128, 0u128, 0u128]
        );
    }
}
