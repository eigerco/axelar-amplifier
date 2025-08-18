use cainome_rs::Abigen;
use std::collections::HashMap;

fn main() {
    let mut aliases = HashMap::new();
    aliases.insert(
        String::from("auth_weighted::component::AuthWeightedComponent::_SignersRotated"),
        String::from("SignersRotated"),
    );
    aliases.insert(
        String::from("gateway::contract::AxelarGateway::_OperatorshipTransferred"),
        String::from("OperatorshipTransferred"),
    );
    aliases.insert(
        String::from("gateway::contract::AxelarGateway::_ContractCall"),
        String::from("ContractCall"),
    );
    aliases.insert(
        String::from("gateway::contract::AxelarGateway::_ContractCallOffchainData"),
        String::from("ContractCallOffchainData"),
    );
    aliases.insert(
        String::from("gateway::contract::AxelarGateway::_MessageExecuted"),
        String::from("MessageExecuted"),
    );
    aliases.insert(
        String::from("gateway::contract::AxelarGateway::_MessageApproved"),
        String::from("MessageApproved"),
    );
    aliases.insert(
        String::from("auth_weighted::component::AuthWeightedComponent::Event"),
        String::from("AuthWeightedEvent"),
    );
    aliases.insert(
        String::from("openzeppelin_access::ownable::ownable::OwnableComponent::Event"),
        String::from("OwnableEvent"),
    );
    aliases.insert(
        String::from("openzeppelin_upgrades::upgradeable::UpgradeableComponent::Event"),
        String::from("UpgradableEvent"),
    );

    let abigen = Abigen::new("AxelarGateway", "./src/cairo_gateway_abi.json")
        .with_types_aliases(aliases)
        .with_derives(vec![
            "Clone".to_string(),
            "Debug".to_string(),
            "PartialEq".to_string(),
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
        ])
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()]);

    abigen
        .generate()
        .expect("Fail to generate bindings")
        .write_to_file("./src/kor.rs")
        .unwrap();
}

// #![allow(
//     missing_docs,
//     reason = "Auto-generated code from Cairo gateway contract ABI"
// )]
//
// use cainome::rs::abigen;
//
// abigen!(
//     AxelarGateway,
//     "./crates/starknet-abigen/abi-files/cairo_gateway_abi.json",
//     type_aliases {
//         auth_weighted::component::AuthWeightedComponent::_SignersRotated as SignersRotated;
//         gateway::contract::AxelarGateway::_OperatorshipTransferred as OperatorshipTransferred;
//         gateway::contract::AxelarGateway::_ContractCall as ContractCall;
//         gateway::contract::AxelarGateway::_ContractCallOffchainData as ContractCallOffchainData;
//         gateway::contract::AxelarGateway::_MessageExecuted as MessageExecuted;
//         gateway::contract::AxelarGateway::_MessageApproved as MessageApproved;
//         auth_weighted::component::AuthWeightedComponent::Event as AuthWeightedEvent;
//         openzeppelin_access::ownable::ownable::OwnableComponent::Event as OwnableEvent;
//         openzeppelin_upgrades::upgradeable::UpgradeableComponent::Event as UpgradableEvent;
//     },
//     derives(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize),
// );
