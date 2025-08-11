use aleo_gmp_types::{GmpAddress, ItsTokenId, SafeGmpChainName};
use aleo_string_encoder::StringEncoder;
use axelar_wasm_std::nonempty;
use error_stack::{bail, report, Report, ResultExt};
use interchain_token_service_std::{HubMessage, InterchainTransfer, Message, TokenId};
use router_api::ChainNameRaw;
use snarkvm_cosmwasm::prelude::{Address, Identifier, Literal, Network, Plaintext};

use crate::aleo::token_id_conversion::ItsTokenIdNewType;
use crate::aleo::Error;

/// Represents an outbound interchain transfer message that is sent to the hub.
///
/// This struct can only be translated to [HubMessage::SendToHub] with [InterchainTransfer] internal message.
#[derive(Debug)]
pub struct ItsOutboundInterchainTransfer<N: Network> {
    /// The inner interchain transfer message containing the transfer details.
    pub inner_message: OutboundInterchainTransfer<N>,
    /// The destination chain where the interchain transfer is directed.
    pub destination_chain: [u128; 2],
}

/// Represents an outbound interchain transfer message that is sent to the hub.
///
/// This struct corresponds to the [InterchainTransfer].
#[derive(Debug)]
pub struct OutboundInterchainTransfer<N: Network> {
    /// Unique identifier for the interchain token
    pub its_token_id: ItsTokenId,
    /// Source address from which the transfer originated.
    pub source_address: Address<N>,
    /// Destination address on the remote chain where the transfer is directed, encoded as a GmpAddress.
    pub destination_address: GmpAddress,
    /// Amount of tokens being transferred, represented as a u128.
    pub amount: u128,
}

impl<N: Network> TryFrom<&Plaintext<N>> for OutboundInterchainTransfer<N> {
    type Error = Report<Error>;

    fn try_from(plaintext: &Plaintext<N>) -> Result<Self, Self::Error> {
        let Plaintext::Struct(map, _) = &plaintext else {
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
                    return Err(report!(Error::TranslationFailed(
                        "Expected to find 'its_token_id' array with exacly two element".to_string()
                    ))
                    .attach_printable(format!("{its_token_id:#?}")));
                }
            }
        };

        let source_address = {
            let key: Identifier<N> = "source_address".parse().map_err(Error::from)?;
            let Some(Plaintext::Literal(Literal::Address(source_address), _)) = map.get(&key)
            else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'source_address'".to_string()
                ));
            };
            *source_address
        };

        let destination_address = {
            let key: Identifier<N> = "destination_address".parse().map_err(Error::from)?;
            let Some(Plaintext::Array(destination_address, _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'destination_address'".to_string()
                ));
            };

            match &destination_address[..] {
                [Plaintext::Literal(Literal::U128(n1), _), Plaintext::Literal(Literal::U128(n2), _), Plaintext::Literal(Literal::U128(n3), _), Plaintext::Literal(Literal::U128(n4), _), Plaintext::Literal(Literal::U128(n5), _), Plaintext::Literal(Literal::U128(n6), _)] => {
                    [**n1, **n2, **n3, **n4, **n5, **n6]
                }
                _ => {
                    return Err(report!(Error::TranslationFailed(
                        "Expected to find 'destination_address' array with exacly six element"
                            .to_string()
                    ))
                    .attach_printable(format!("{destination_address:#?}")));
                }
            }
        };
        let amount = {
            let key: Identifier<N> = "amount".parse().map_err(Error::from)?;
            let Some(Plaintext::Literal(Literal::U128(amount), _)) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'amount' as u128".to_string()
                ));
            };

            **amount
        };

        Ok(OutboundInterchainTransfer {
            its_token_id,
            source_address,
            destination_address,
            amount,
        })
    }
}

impl<N: Network> TryFrom<&Plaintext<N>> for ItsOutboundInterchainTransfer<N> {
    type Error = Report<Error>;

    fn try_from(value: &Plaintext<N>) -> Result<Self, Self::Error> {
        let Plaintext::Struct(map, _) = value else {
            bail!(Error::TranslationFailed(
                "Expected a Plaintext::Struct".to_string()
            ));
        };

        let inner_message = {
            let key: Identifier<N> = "inner_message".parse().map_err(Error::from)?;
            let Some(nested_plaintext) = map.get(&key) else {
                bail!(Error::TranslationFailed(
                    "Expected to find 'inner_message'".to_string()
                ));
            };

            OutboundInterchainTransfer::<N>::try_from(nested_plaintext).attach_printable_lazy(
                || format!("Failed to parse 'inner_message': {nested_plaintext:#?}"),
            )?
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
                    return Err(report!(Error::TranslationFailed(
                        "Expected to find 'destination_chain' array with exactly two elements"
                            .to_string()
                    ))
                    .attach_printable(format!("{destination_chain:#?}")));
                }
            }
        };

        Ok(ItsOutboundInterchainTransfer {
            inner_message,
            destination_chain,
        })
    }
}

impl<N: Network> TryFrom<&OutboundInterchainTransfer<N>> for InterchainTransfer {
    type Error = Report<Error>;

    fn try_from(outbound_transfer: &OutboundInterchainTransfer<N>) -> Result<Self, Self::Error> {
        let its_token_id = ItsTokenIdNewType(outbound_transfer.its_token_id);
        let source_address =
            StringEncoder::encode_string(&outbound_transfer.source_address.to_string())
                .map_err(Error::from)?
                .decode()
                .map_err(Error::from)?
                .into_bytes()
                .try_into()
                .map_err(Error::from)?;

        let destination_address = {
            let destination_address =
                StringEncoder::from_slice(&outbound_transfer.destination_address)
                    .decode()
                    .map_err(Error::from)?;

            let destination_address = destination_address
                .strip_prefix("0x")
                .unwrap_or(&destination_address);

            nonempty::HexBinary::try_from(hex::decode(destination_address).map_err(Error::from)?)
                .map_err(Error::from)?
        };

        Ok(InterchainTransfer {
            token_id: TokenId::from(its_token_id),
            source_address,
            destination_address,
            amount: cosmwasm_std::Uint256::from_u128(outbound_transfer.amount)
                .try_into()
                .map_err(Error::from)?,
            data: None,
        })
    }
}

impl<N: Network> TryFrom<ItsOutboundInterchainTransfer<N>> for HubMessage {
    type Error = Report<Error>;

    fn try_from(outbound_transfer: ItsOutboundInterchainTransfer<N>) -> Result<Self, Self::Error> {
        let interchain_transfer = InterchainTransfer::try_from(&outbound_transfer.inner_message)
            .attach_printable_lazy(|| format!("{:#?}", outbound_transfer.inner_message))?;

        let destination_chain: SafeGmpChainName =
            SafeGmpChainName::new(outbound_transfer.destination_chain).change_context_lazy(
                || Error::TranslationFailed("Failed to translate chain name".to_string()),
            )?;

        Ok(HubMessage::SendToHub {
            destination_chain: ChainNameRaw::try_from(destination_chain).map_err(Error::from)?,
            message: Message::InterchainTransfer(interchain_transfer),
        })
    }
}
