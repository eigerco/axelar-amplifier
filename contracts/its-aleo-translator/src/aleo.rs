use aleo_gateway::types::SafeGmpChainName;
use axelar_wasm_std::{nonempty, IntoContractError};
use cosmwasm_std::{ConversionOverflowError, HexBinary, StdError};
use error_stack::{bail, report, Report};
use inbound_deploy_interchain_token::{
    FromRemoteDeployInterchainToken, ItsMessageDeployInterchainToken,
};
use inbound_transfers::{IncomingInterchainTransfer, ItsIncomingInterchainTransfer};
use interchain_token_service_std::{HubMessage, Message};
use outbound_deploy_interchain_token::RemoteDeployInterchainToken;
use outbound_transfers::ItsOutgoingInterchainTransfer;
use plaintext_trait::ToPlaintext;
use snarkvm_cosmwasm::prelude::{FromBytes as _, Network, Value};
use thiserror::Error;

mod inbound_deploy_interchain_token;
mod inbound_transfers;
mod outbound_deploy_interchain_token;
mod outbound_transfers;
mod token_id_conversion;

#[derive(Error, Debug, IntoContractError)]
pub enum Error {
    #[error(transparent)]
    Std(#[from] StdError),
    #[error("SnarkVmError: {0}")]
    SnarkVm(#[from] snarkvm_cosmwasm::prelude::Error),
    #[error("StringEncoder: {0}")]
    StringEncoder(#[from] aleo_string_encoder::Error),
    #[error("Utf8Error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Aleo: {0}")]
    AleoGateway(#[from] aleo_gateway::error::AleoError),
    #[error(transparent)]
    NonEmpty(#[from] nonempty::Error),
    #[error("TranslationFailed: {0}")]
    TranslationFailed(String),
    #[error("RouterApi: {0}")]
    RouterApi(#[from] router_api::error::Error),
    #[error("ConversionOverflowError: {0}")]
    ConversionOverflow(#[from] ConversionOverflowError),
    #[error("Hex: {0}")]
    Hex(#[from] hex::FromHexError),
}

/// Converts a HubMessage to snarkvm Value.
pub fn aleo_inbound_hub_message<N: Network>(
    hub_message: HubMessage,
) -> Result<Value<N>, Report<Error>> {
    match hub_message {
        HubMessage::ReceiveFromHub {
            source_chain,
            message: Message::InterchainTransfer(interchain_transfer),
        } => {
            let source_chain: SafeGmpChainName =
                SafeGmpChainName::try_from(source_chain).map_err(|e| {
                    report!(Error::TranslationFailed(format!(
                        "Failed to convert source chain to AleoGmpChainName: {e}"
                    )))
                })?;

            let aleo_inbound_transfer =
                IncomingInterchainTransfer::<N>::try_from(&interchain_transfer).map_err(|e| {
                    report!(Error::TranslationFailed(format!(
                        "Failed to convert InterchainTransfer to IncomingInterchainTransfer: {e}"
                    )))
                })?;

            let message = ItsIncomingInterchainTransfer::<N> {
                inner_message: aleo_inbound_transfer,
                source_chain: source_chain.chain_name(),
            };

            let aleo_plaintext = message.to_plaintext();
            Ok(snarkvm_cosmwasm::prelude::Value::Plaintext(aleo_plaintext))
        }
        HubMessage::ReceiveFromHub {
            source_chain,
            message: Message::DeployInterchainToken(deploy_interchain_token),
        } => {
            let source_chain: SafeGmpChainName =
                SafeGmpChainName::try_from(source_chain).map_err(|e| {
                    report!(Error::TranslationFailed(format!(
                        "Failed to convert source chain to AleoGmpChainName: {e}"
                    )))
                })?;

            let message = ItsMessageDeployInterchainToken::<N> {
                inner_message: FromRemoteDeployInterchainToken::try_from(deploy_interchain_token)?,
                source_chain: source_chain.chain_name(),
            };

            let aleo_plaintext = message.to_plaintext();
            Ok(snarkvm_cosmwasm::prelude::Value::Plaintext(aleo_plaintext))
        }
        _ => bail!(Error::TranslationFailed(format!(
            "Unsupported HubMessage type for inbound translation: {hub_message:?}"
        ))),
    }
}

/// Converts an ITS HubMessage to snarkvm Value.
pub fn aleo_outbound_hub_message<N: Network>(
    payload: HexBinary,
) -> Result<HubMessage, Report<Error>> {
    let value = Value::<N>::from_bytes_le(&payload).map_err(|e| report!(Error::SnarkVm(e)))?;

    let Value::Plaintext(plaintext) = value else {
        bail!(Error::TranslationFailed(
            "Expected a Plaintext value".to_string()
        ));
    };

    if let Ok(its_outgoing_transfer) = ItsOutgoingInterchainTransfer::<N>::try_from(&plaintext) {
        Ok(HubMessage::try_from(its_outgoing_transfer)?)
    } else if let Ok(remote_deploy_interchain_token) =
        RemoteDeployInterchainToken::try_from(&plaintext)
    {
        Ok(HubMessage::try_from(remote_deploy_interchain_token)?)
    } else {
        bail!(Error::TranslationFailed(
            format!("Failed to convert Plaintext to ItsOutgoingInterchainTransfer or RemoteDeployInterchainToken. Received plaintext: {plaintext:?}")
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use aleo_gateway::types::GmpChainName;
    use aleo_string_encoder::StringEncoder;
    use interchain_token_service_std::{InterchainTransfer, Message, TokenId};
    use router_api::ChainNameRaw;
    use snarkvm_cosmwasm::prelude::{Address, ToBytes as _};

    use super::*;
    use crate::aleo::token_id_conversion::ItsTokenIdNewType;

    type CurrentNetwork = snarkvm_cosmwasm::prelude::TestnetV0;

    const EVM_DESTINATION_ADDRESS: &str = "aA411dE17e8E5C12bfac98c53670D520BB827d94";
    const ETH_SEPOLIA_CHAIN_ID: [u128; 2] = [134856446981446044681495648599413882880u128, 0u128];
    const EVM_ADDRESS_AS_ALEO: [u128; 6] = [
        129273673469706941367715161866217140530u128,
        130795933121520764988248272954977497648u128,
        88072879107956514698150556559508242432u128,
        0u128,
        0u128,
        0u128,
    ];

    fn random_address() -> String {
        rand::random::<Address<CurrentNetwork>>().to_string()
    }

    fn random_token_id() -> TokenId {
        rand::random::<[u8; 32]>().into()
    }

    fn eth_sepolia_chain() -> ChainNameRaw {
        ChainNameRaw::from_str("eth-sepolia").unwrap()
    }

    fn from_hex(hex: &str) -> nonempty::HexBinary {
        HexBinary::from_hex(hex)
            .expect("Valid hex string")
            .try_into()
            .expect("Valid non-empty hex binary")
    }

    struct TestTransferBuilder {
        token_id: TokenId,
        source_address: String,
        destination_address: String,
        amount: u64,
        chain: ChainNameRaw,
    }

    impl TestTransferBuilder {
        fn new() -> Self {
            Self {
                token_id: random_token_id(),
                source_address: random_address(),
                destination_address: EVM_DESTINATION_ADDRESS.to_string(),
                amount: rand::random(),
                chain: eth_sepolia_chain(),
            }
        }

        fn build_hub_message(self) -> HubMessage {
            HubMessage::SendToHub {
                destination_chain: self.chain,
                message: Message::InterchainTransfer(InterchainTransfer {
                    token_id: self.token_id,
                    source_address: from_hex(&hex::encode(&self.source_address)),
                    destination_address: from_hex(&self.destination_address),
                    amount: self.amount.try_into().expect("Valid amount"),
                    data: None,
                }),
            }
        }
    }

    fn format_aleo_transfer(
        token_id: &ItsTokenIdNewType,
        source_address: &str,
        amount: u64,
    ) -> String {
        format!(
            r#"{{
            inner_message: {{
                its_token_id: [ {}u128, {}u128 ],
                source_address: {},
                destination_address: [ {}u128, {}u128, {}u128, {}u128, {}u128, {}u128 ],
                amount: {}u128
            }},
            destination_chain: [ {}u128, {}u128 ]
        }}"#,
            token_id[0],
            token_id[1],
            source_address,
            EVM_ADDRESS_AS_ALEO[0],
            EVM_ADDRESS_AS_ALEO[1],
            EVM_ADDRESS_AS_ALEO[2],
            EVM_ADDRESS_AS_ALEO[3],
            EVM_ADDRESS_AS_ALEO[4],
            EVM_ADDRESS_AS_ALEO[5],
            amount,
            ETH_SEPOLIA_CHAIN_ID[0],
            ETH_SEPOLIA_CHAIN_ID[1]
        )
    }

    #[test]
    fn translate_outbound_transfer() {
        let test_data = TestTransferBuilder::new();

        // Create Aleo value representation
        let its_token_id = ItsTokenIdNewType::from(test_data.token_id);
        let aleo_value_str = format_aleo_transfer(&its_token_id, &test_data.source_address, test_data.amount);

        let aleo_value = Value::<CurrentNetwork>::from_str(&aleo_value_str)
            .expect("Valid Aleo value")
            .to_bytes_le()
            .expect("Serializable to bytes");

        let result = aleo_outbound_hub_message::<CurrentNetwork>(HexBinary::from(aleo_value))
            .expect("Successful conversion");

        let expected_message = test_data.build_hub_message();
        assert_eq!(result, expected_message);
    }

    #[test]
    fn translate_inbound_transfer() {
        let aleo_source_address = rand::random::<Address<CurrentNetwork>>().to_string();
        let expected_aleo_message = format!(
            r#"{{
  inner_message: {{
    its_token_id: [
      1334440654591915542993625911497130241u128,
      1334440654591915542993625911497130241u128
    ],
    source_address: [
      129273673469706941367715161866217140530u128,
      130795933121520764988248272954977497648u128,
      88072879107956514698150556559508242432u128,
      0u128,
      0u128,
      0u128
    ],
    destination_address: {aleo_source_address},
    amount: 100u128
  }},
  source_chain: [
    134856446981446044681495648599413882880u128,
    0u128
  ]
}}"#
        );

        let its_message = HubMessage::ReceiveFromHub {
            source_chain: eth_sepolia_chain(),
            message: Message::InterchainTransfer(InterchainTransfer {
                token_id: TokenId::from([1u8; 32]),
                source_address: from_hex(hex::encode(EVM_DESTINATION_ADDRESS).as_str()),
                destination_address: from_hex(hex::encode(aleo_source_address).as_str()),
                amount: 100u64.try_into().unwrap(),
                data: None,
            }),
        };

        let aleo_message = aleo_inbound_hub_message::<CurrentNetwork>(its_message)
            .expect("Failed to convert HubMessage to Aleo value");

        let exected_aleo_value = Value::<CurrentNetwork>::from_str(&expected_aleo_message)
            .expect("Failed to parse Aleo value");
        assert_eq!(
            aleo_message,
            exected_aleo_value,
            "Expected Aleo value does not match the actual Aleo value. \nExpected: {:?}\nActual: {:?}",
            exected_aleo_value, aleo_message
        );
    }

    #[test]
    fn translate_inbound_token_deploy() {
        let minter_address = rand::random::<Address<CurrentNetwork>>().to_string();
        let source_chain = eth_sepolia_chain();
        let aleo_source_chain = SafeGmpChainName::try_from(&source_chain)
            .unwrap()
            .chain_name();
        let token_id: TokenId = rand::random::<[u8; 32]>().into();
        let suffix: u8 = rand::random();
        let token_name = format!("TokenName_{suffix}");
        let token_symbol = format!("TN{suffix}");
        let decimals: u8 = rand::random();
        let aleo_token_name = StringEncoder::encode_string(&token_name).unwrap().consume()[0];
        let aleo_token_symbol = StringEncoder::encode_string(&token_symbol)
            .unwrap()
            .consume()[0];

        let its_token_id = ItsTokenIdNewType::from(token_id);

        let its_hub_message = HubMessage::ReceiveFromHub {
            source_chain: source_chain.clone(),
            message: Message::DeployInterchainToken(
                interchain_token_service_std::DeployInterchainToken {
                    token_id,
                    name: token_name.as_str().try_into().unwrap(),
                    symbol: token_symbol.as_str().try_into().unwrap(),
                    decimals,
                    minter: None,
                },
            ),
        };

        let aleo_value = aleo_inbound_hub_message::<CurrentNetwork>(its_hub_message)
            .expect("Failed to convert HubMessage to Aleo value");
        let expected = format!(
            "{{
                inner_message: {{
                    its_token_id: [
                        {}u128,
                        {}u128
                    ],
                    name: {aleo_token_name}u128,
                    symbol: {aleo_token_symbol}u128,
                    decimals: {decimals}u8,
                    minter: {}
                }},
                source_chain: [
                {}u128,
                {}u128
                ]
            }}",
            its_token_id[0],
            its_token_id[1],
            Address::<CurrentNetwork>::zero(),
            aleo_source_chain[0],
            aleo_source_chain[1]
        );
        let expected_aleo_value = Value::<CurrentNetwork>::from_str(&expected)
            .expect("Failed to parse expected Aleo value");

        let its_hub_message = HubMessage::ReceiveFromHub {
            source_chain,
            message: Message::DeployInterchainToken(
                interchain_token_service_std::DeployInterchainToken {
                    token_id,
                    name: token_name.try_into().unwrap(),
                    symbol: token_symbol.try_into().unwrap(),
                    decimals,
                    minter: Some(from_hex(hex::encode(&minter_address).as_str())),
                },
            ),
        };
        assert_eq!(
            aleo_value, expected_aleo_value,
            "Expected Aleo value does not match the actual Aleo value. \nExpected: {:?}\nActual: {:?}",
            expected_aleo_value, aleo_value
        );

        let aleo_value_with_minter = aleo_inbound_hub_message::<CurrentNetwork>(its_hub_message)
            .expect("Failed to convert HubMessage to Aleo value");
        let expected = format!(
            "{{
                inner_message: {{
                    its_token_id: [
                        {}u128,
                        {}u128
                    ],
                    name: {aleo_token_name}u128,
                    symbol: {aleo_token_symbol}u128,
                    decimals: {decimals}u8,
                    minter: {}
                }},
                source_chain: [
                {}u128,
                {}u128
                ]
            }}",
            its_token_id[0],
            its_token_id[1],
            minter_address,
            aleo_source_chain[0],
            aleo_source_chain[1]
        );
        let expected_aleo_value = Value::<CurrentNetwork>::from_str(&expected)
            .expect("Failed to parse expected Aleo value");
        assert_eq!(
            aleo_value_with_minter, expected_aleo_value,
            "Expected Aleo value with minter does not match the actual Aleo value. \nExpected: {:?}\nActual: {:?}",
            expected_aleo_value, aleo_value_with_minter
        );
    }

    // fn format_aleo_deploy_interchain_token(
    //     its_token_id: &ItsTokenIdNewType,
    //     token_name: u128,
    //     token_symbol: u128,
    //     decimals: u8,
    //     minter: Option<&str>,
    //     destination_chain: &GmpChainName,
    // ) -> String {
    //     let minter_str = match minter {
    //         Some(m) => format!("[ {}, {}, {} ]", m, 0u128, 0u128),
    //         None => "[ 0u128, 0u128, 0u128, 0u128, 0u128, 0u128 ]".to_string(),
    //     };
    //     format!(
    //         r#"{{
    //             payload: {{
    //                 its_token_id: [ {}u128, {}u128 ],
    //                 name: {token_name}u128,
    //                 symbol: {token_symbol}u128,
    //                 decimals: {decimals}u8,
    //                 minter: {minter_str}
    //             }},
    //             destination_chain: [ {}u128, {}u128 ]
    //         }}"#,
    //         its_token_id[0],
    //         its_token_id[1],
    //         destination_chain[0],
    //         destination_chain[1]
    //     )
    // }

    struct TestDeployBuilder {
        token_id: TokenId,
        token_name: String,
        token_symbol: String,
        decimals: u8,
        minter: Option<String>,
        destination_chain: ChainNameRaw,
    }

    impl TestDeployBuilder {
        fn new() -> Self {
            let token_id = random_token_id();
            let suffix: u8 = rand::random();
            let token_name = format!("TokenName_{suffix}");
            let token_symbol = format!("TN{suffix}");
            let decimals: u8 = rand::random();
            let minter = Some(EVM_DESTINATION_ADDRESS.to_string());
            let destination_chain = eth_sepolia_chain();

            Self {
                token_id,
                token_name,
                token_symbol,
                decimals,
                minter,
                destination_chain,
            }
        }
    }

    fn format_aleo_deploy_interchain_token(
        test_builder: &TestDeployBuilder,
        its_token_id: &ItsTokenIdNewType,
        token_name: u128,
        token_symbol: u128,
        decimals: u8,
        minter: Option<&str>,
        destination_chain: &GmpChainName,
    ) -> String {
        let minter_str = match minter {
            Some(m) => format!("[ {}, {}, {} ]", m, 0u128, 0u128),
            None => "[ 0u128, 0u128, 0u128, 0u128, 0u128, 0u128 ]".to_string(),
        };
        format!(
            r#"{{
                payload: {{
                    its_token_id: [ {}u128, {}u128 ],
                    name: {token_name}u128,
                    symbol: {token_symbol}u128,
                    decimals: {decimals}u8,
                    minter: {minter_str}
                }},
                destination_chain: [ {}u128, {}u128 ]
            }}"#,
            its_token_id[0],
            its_token_id[1],
            destination_chain[0],
            destination_chain[1]
        )
    }

    #[test]
    fn translate_outbound_token_deploy() {
        let evm_minter_address = EVM_DESTINATION_ADDRESS;
        let destination_chain = eth_sepolia_chain();
        let aleo_destination_chain = SafeGmpChainName::try_from(&destination_chain)
            .expect("Failed to convert destination chain to SafeGmpChainName")
            .chain_name();
        let suffix: u8 = rand::random();
        let token_name = format!("TokenName_{suffix}");
        let token_symbol = format!("TN{suffix}");
        let decimals: u8 = rand::random();
        let aleo_token_name = StringEncoder::encode_string(&token_name).unwrap().consume()[0];
        let aleo_token_symbol = StringEncoder::encode_string(&token_symbol)
            .unwrap()
            .consume()[0];

        let outbound_deploy_interchain_token = format_aleo_deploy_interchain_token(
            &its_token_id,
            aleo_token_name,
            aleo_token_symbol,
            decimals,
            None,
            &aleo_destination_chain,
        );

        let aleo_value = Value::<CurrentNetwork>::from_str(&outbound_deploy_interchain_token)
            .expect("Failed to parse Aleo value")
            .to_bytes_le()
            .expect("Failed to convert Aleo value to bytes");
        let aleo_value = HexBinary::from(aleo_value);
        let its_hub_message = aleo_outbound_hub_message::<CurrentNetwork>(aleo_value)
            .expect("Failed to convert Aleo value to HubMessage");

        let expected_message = HubMessage::SendToHub {
            destination_chain: destination_chain.clone(),
            message: Message::DeployInterchainToken(
                interchain_token_service_std::DeployInterchainToken {
                    token_id,
                    name: token_name.clone().try_into().unwrap(),
                    symbol: token_symbol.clone().try_into().unwrap(),
                    decimals,
                    minter: None,
                },
            ),
        };

        assert_eq!(its_hub_message, expected_message,
            "Expected HubMessage does not match the actual HubMessage. \nExpected: {:?}\nActual: {:?}",
            expected_message, its_hub_message
        );

        let outbound_deploy_interchain_token = format!(
            "{{
                payload: {{
                    its_token_id: [
                        {}u128,
                        {}u128
                    ],
                    name: {aleo_token_name}u128,
                    symbol: {aleo_token_symbol}u128,
                    decimals: {decimals}u8,
                    minter: [ {}u128, {}u128, {}u128, 0u128, 0u128, 0u128 ]
                }},
                destination_chain: [ {}u128, {}u128 ]
            }}",
            its_token_id[0],
            its_token_id[1],
            aleo_encoded_minter_address[0],
            aleo_encoded_minter_address[1],
            aleo_encoded_minter_address[2],
            aleo_destination_chain[0],
            aleo_destination_chain[1],
        );
        let aleo_value = Value::<CurrentNetwork>::from_str(&outbound_deploy_interchain_token)
            .expect("Failed to parse Aleo value")
            .to_bytes_le()
            .expect("Failed to convert Aleo value to bytes");
        let aleo_value = HexBinary::from(aleo_value);
        let its_hub_message = aleo_outbound_hub_message::<CurrentNetwork>(aleo_value)
            .expect("Failed to convert Aleo value to HubMessage");

        let expected_message = HubMessage::SendToHub {
            destination_chain,
            message: Message::DeployInterchainToken(
                interchain_token_service_std::DeployInterchainToken {
                    token_id,
                    name: token_name.try_into().unwrap(),
                    symbol: token_symbol.try_into().unwrap(),
                    decimals,
                    minter: Some(from_hex(evm_minter_address)),
                },
            ),
        };

        assert_eq!(its_hub_message, expected_message,
            "Expected HubMessage does not match the actual HubMessage. \nExpected: {:?}\nActual: {:?}",
            expected_message, its_hub_message
        );
    }
}
