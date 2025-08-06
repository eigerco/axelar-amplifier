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
            "Failed to convert Plaintext to ItsOutgoingInterchainTransfer or RemoteDeployInterchainToken".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use aleo_string_encoder::StringEncoder;
    use interchain_token_service_std::{InterchainTransfer, Message, TokenId};
    use router_api::ChainNameRaw;
    use snarkvm_cosmwasm::prelude::{Address, ToBytes as _};

    use super::*;
    use crate::aleo::token_id_conversion::ItsTokenIdNewType;

    type CurrentNetwork = snarkvm_cosmwasm::prelude::TestnetV0;

    fn from_hex(hex: &str) -> nonempty::HexBinary {
        HexBinary::from_hex(hex).unwrap().try_into().unwrap()
    }

    #[test]
    fn translate_outbound_transfer() {
        let evm_destination_address = "aA411dE17e8E5C12bfac98c53670D520BB827d94";
        let aleo_source_address = rand::random::<Address<CurrentNetwork>>().to_string();
        let amount: u64 = rand::random();
        let token_id: TokenId = rand::random::<[u8; 32]>().into();
        let its_token_id = ItsTokenIdNewType::from(token_id);

        let its_outgoing_interchain_transfer = format!(
            r#"{{
            inner_message: {{
                its_token_id: [ {}u128, {}u128 ],
                source_address: {aleo_source_address},
                destination_address: [129273673469706941367715161866217140530u128, 130795933121520764988248272954977497648u128, 88072879107956514698150556559508242432u128, 0u128, 0u128, 0u128],
                amount: {amount}u128
            }},
            destination_chain: [ 134856446981446044681495648599413882880u128, 0u128 ]
        }}"#,
            its_token_id[0], its_token_id[1],
        );

        let aleo_value = Value::<CurrentNetwork>::from_str(&its_outgoing_interchain_transfer)
            .expect("Failed to parse Aleo value")
            .to_bytes_le()
            .expect("Failed to convert Aleo value to bytes");

        let aleo_value = HexBinary::from(aleo_value);

        let its_hub_message = aleo_outbound_hub_message::<CurrentNetwork>(aleo_value)
            .expect("Failed to convert Aleo value to HubMessage");

        let expected_message = HubMessage::SendToHub {
            destination_chain: ChainNameRaw::from_str("eth-sepolia").unwrap(),
            message: Message::InterchainTransfer(InterchainTransfer {
                token_id,
                source_address: from_hex(hex::encode(aleo_source_address).as_str()),
                destination_address: from_hex(evm_destination_address),
                amount: amount.try_into().unwrap(),
                data: None,
            }),
        };

        assert_eq!(its_hub_message, expected_message,
            "Expected HubMessage does not match the actual HubMessage. \nExpected: {:?}\nActual: {:?}",
            expected_message, its_hub_message
        );
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
            source_chain: ChainNameRaw::from_str("eth-sepolia").unwrap(),
            message: Message::InterchainTransfer(InterchainTransfer {
                token_id: TokenId::from([1u8; 32]),
                source_address: from_hex(
                    hex::encode("aA411dE17e8E5C12bfac98c53670D520BB827d94").as_str(),
                ),
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
        let source_chain = ChainNameRaw::from_str("eth-sepolia").unwrap();
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

    #[test]
    fn translate_outbound_token_deploy() {
        let evm_minter_address = "aA411dE17e8E5C12bfac98c53670D520BB827d94";
        let aleo_encoded_minter_address = StringEncoder::encode_string(evm_minter_address)
            .unwrap()
            .consume();
        let destination_chain = ChainNameRaw::from_str("eth-sepolia").unwrap();
        let aleo_destination_chain = SafeGmpChainName::try_from(&destination_chain)
            .unwrap()
            .chain_name();
        let token_id: TokenId = rand::random::<[u8; 32]>().into();
        let its_token_id = ItsTokenIdNewType::from(token_id);
        let suffix: u8 = rand::random();
        let token_name = format!("TokenName_{suffix}");
        let token_symbol = format!("TN{suffix}");
        let decimals: u8 = rand::random();
        let aleo_token_name = StringEncoder::encode_string(&token_name).unwrap().consume()[0];
        let aleo_token_symbol = StringEncoder::encode_string(&token_symbol)
            .unwrap()
            .consume()[0];

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
                    minter: [ 0u128, 0u128, 0u128, 0u128, 0u128, 0u128 ]
                }},
                destination_chain: [ {}u128, {}u128 ]
            }}",
            its_token_id[0], its_token_id[1], aleo_destination_chain[0], aleo_destination_chain[1]
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
