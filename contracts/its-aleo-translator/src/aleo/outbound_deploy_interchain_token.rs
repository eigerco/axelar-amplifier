use std::str::FromStr as _;

use aleo_gateway::types::{GmpAddress, GmpChainName, ItsTokenId};
use aleo_string_encoder::StringEncoder;
use axelar_wasm_std::nonempty;
use error_stack::{bail, report, Report, ResultExt};
use interchain_token_service_std::{DeployInterchainToken as DeployInterchainTokenItsHub, TokenId};
use interchain_token_service_std::{HubMessage, Message};
use plaintext_trait::ToPlaintext;
use router_api::ChainNameRaw;
use snarkvm_cosmwasm::prelude::{Identifier, Literal, Network, Plaintext};

use super::token_id_conversion::ItsTokenIdNewType;
use super::Error;


/// Represents a deploy interchain token message that is sent to the hub.
///
/// It can only be translated to
/// [HubMessage::SendToHub]
/// with [DeployInterchainToken](interchain_token_service_std::DeployInterchainToken)
#[derive(ToPlaintext, Clone, Debug)]
pub struct RemoteDeployInterchainToken {
    pub payload: DeployInterchainToken,
    pub destination_chain: GmpChainName,
}

/// Represents a deploy interchain token message that is sent to the hub.
///
/// This struct corresponds to the [DeployInterchainToken](interchain_token_service_std::primitives::DeployInterchainToken).
/// Because on Aleo there are no optional values, an array with zero values is used to represent
/// the absence of a minter address.
#[derive(ToPlaintext, Clone, Debug)]
pub struct DeployInterchainToken {
    /// Unique identifier for the interchain token
    pub its_token_id: ItsTokenId,
    /// Token name encoded as a u128 value
    pub name: u128,
    /// Token symbol encoded as a u128 value
    pub symbol: u128,
    /// Number of decimal places for the token
    pub decimals: u8,
    /// Address authorized to mint tokens (`[0, 0, 0, 0, 0, 0]` if no minter)
    pub minter: GmpAddress,
}

impl TryFrom<DeployInterchainToken> for DeployInterchainTokenItsHub {
    type Error = Report<Error>;

    fn try_from(message: DeployInterchainToken) -> Result<Self, Self::Error> {
        let name = StringEncoder::from_slice(&[message.name]);
        let symbol = StringEncoder::from_slice(&[message.symbol]);
        let its_token_id = ItsTokenIdNewType(message.its_token_id);

        let minter = {
            let minter = StringEncoder::from_slice(&message.minter)
                .decode()
                .map_err(Error::from)?;

            let minter = minter.strip_prefix("0x").unwrap_or(&minter);

            if minter.is_empty() {
                None
            } else {
                Some(
                    nonempty::HexBinary::try_from(hex::decode(minter).map_err(Error::from)?)
                        .map_err(Error::from)?,
                )
            }
        };

        Ok(DeployInterchainTokenItsHub {
            token_id: TokenId::from(its_token_id),
            name: axelar_wasm_std::nonempty::String::try_from(name.decode().map_err(Error::from)?)
                .map_err(Error::from)?,
            symbol: axelar_wasm_std::nonempty::String::try_from(
                symbol.decode().map_err(Error::from)?,
            )
            .map_err(Error::from)?,
            decimals: message.decimals,
            minter,
        })
    }
}

impl TryFrom<RemoteDeployInterchainToken> for HubMessage {
    type Error = Report<Error>;

    fn try_from(transfer: RemoteDeployInterchainToken) -> Result<Self, Self::Error> {
        let deploy_interchain_token = DeployInterchainTokenItsHub::try_from(transfer.payload)?;

        let destination_chain = StringEncoder::from_slice(&transfer.destination_chain)
            .decode()
            .map_err(|e| {
                report!(Error::TranslationFailed(format!(
                    "Failed to decode destination chain: {e}"
                )))
            })?;

        let message_hub = HubMessage::SendToHub {
            destination_chain: ChainNameRaw::from_str(&destination_chain)
                .map_err(Error::from)
                .attach_printable_lazy(|| destination_chain.to_string())?,
            message: Message::DeployInterchainToken(deploy_interchain_token),
        };

        Ok(message_hub)
    }
}

impl<N: Network> TryFrom<&Plaintext<N>> for DeployInterchainToken {
    type Error = Report<Error>;

    fn try_from(value: &Plaintext<N>) -> Result<Self, Self::Error> {
        let Plaintext::Struct(map, _) = value else {
            bail!(Error::TranslationFailed(
                "Expected a Plaintext::Struct".to_string()
            ));
        };

        let its_token_id = {
            let key: Identifier<N> = "its_token_id".parse().map_err(Error::from)?;
            let Some(Plaintext::Array(its_token_id, _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'its_token_id' array".to_string()
                ));
            };

            match &its_token_id[..] {
                [Plaintext::Literal(Literal::U128(n1), _), Plaintext::Literal(Literal::U128(n2), _)] => {
                    [**n1, **n2]
                }
                _ => {
                    bail!(Error::TranslationFailed(
                        "Expected to find 'its_token_id' array with exactly two elements"
                            .to_string()
                    ));
                }
            }
        };

        let name = {
            let key: Identifier<N> = "name".parse().map_err(Error::from)?;
            let Some(Plaintext::Literal(Literal::U128(name), _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'name'".to_string()
                ));
            };

            **name
        };

        let symbol = {
            let key: Identifier<N> = "symbol".parse().map_err(Error::from)?;
            let Some(Plaintext::Literal(Literal::U128(symbol), _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'symbol'".to_string()
                ));
            };

            **symbol
        };

        let decimals = {
            let key: Identifier<N> = "decimals".parse().map_err(Error::from)?;
            let Some(Plaintext::Literal(Literal::U8(decimals), _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'decimals'".to_string()
                ));
            };

            **decimals
        };

        let minter = {
            let key: Identifier<N> = "minter".parse().map_err(Error::from)?;
            let Some(Plaintext::Array(minter, _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'minter'".to_string()
                ));
            };

            match &minter[..] {
                [Plaintext::Literal(Literal::U128(n1), _), Plaintext::Literal(Literal::U128(n2), _), Plaintext::Literal(Literal::U128(n3), _), Plaintext::Literal(Literal::U128(n4), _), Plaintext::Literal(Literal::U128(n5), _), Plaintext::Literal(Literal::U128(n6), _)] => {
                    [**n1, **n2, **n3, **n4, **n5, **n6]
                }
                _ => {
                    return Err(report!(Error::TranslationFailed(
                        "Expected to find 'minter' array with exactly six elements".to_string()
                    ))
                    .attach_printable(format!("{minter:#?}")));
                }
            }
        };

        Ok(DeployInterchainToken {
            its_token_id,
            name,
            symbol,
            decimals,
            minter,
        })
    }
}

impl<N: Network> TryFrom<&Plaintext<N>> for RemoteDeployInterchainToken {
    type Error = Report<Error>;

    fn try_from(value: &Plaintext<N>) -> Result<Self, Self::Error> {
        let Plaintext::Struct(map, _) = value else {
            bail!(Error::TranslationFailed(
                "Expected a Plaintext::Struct".to_string()
            ));
        };

        let payload = {
            let key: Identifier<N> = "payload".parse().map_err(Error::from)?;
            let Some(nested_plaintext) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'payload'".to_string()
                ));
            };

            DeployInterchainToken::try_from(nested_plaintext)?
        };

        let destination_chain = {
            let key: Identifier<N> = "destination_chain".parse().map_err(Error::from)?;
            let Some(Plaintext::Array(destination_chain, _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'destination_chain'".to_string()
                ));
            };

            match &destination_chain[..] {
                [Plaintext::Literal(Literal::U128(n1), _), Plaintext::Literal(Literal::U128(n2), _)] => {
                    [**n1, **n2]
                }
                _ => {
                    bail!(Error::TranslationFailed(
                        "Expected to find 'destination_chain' array with exactly two elements"
                            .to_string()
                    ));
                }
            }
        };

        Ok(RemoteDeployInterchainToken {
            payload,
            destination_chain,
        })
    }
}
