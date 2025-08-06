use aleo_gateway::types::{GmpAddress, ItsTokenId, SafeGmpChainName};
use aleo_string_encoder::StringEncoder;
use axelar_wasm_std::nonempty;
use error_stack::{bail, report, Report, ResultExt};
use interchain_token_service_std::{HubMessage, InterchainTransfer, Message, TokenId};
use router_api::ChainNameRaw;
use snarkvm_cosmwasm::prelude::{Address, Identifier, Literal, Network, Plaintext};

use crate::aleo::token_id_conversion::ItsTokenIdNewType;
use crate::aleo::Error;

#[derive(Debug)]
pub struct OutgoingInterchainTransfer<N: Network> {
    pub its_token_id: ItsTokenId,
    pub source_address: Address<N>,
    pub destination_address: GmpAddress,
    pub amount: u128,
}

#[derive(Debug)]
pub struct ItsOutgoingInterchainTransfer<N: Network> {
    pub inner_message: OutgoingInterchainTransfer<N>,
    pub destination_chain: [u128; 2],
}

impl<N: Network> TryFrom<&Plaintext<N>> for OutgoingInterchainTransfer<N> {
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

        Ok(OutgoingInterchainTransfer {
            its_token_id,
            source_address,
            destination_address,
            amount,
        })
    }
}

impl<N: Network> TryFrom<&Plaintext<N>> for ItsOutgoingInterchainTransfer<N> {
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

            OutgoingInterchainTransfer::<N>::try_from(nested_plaintext).attach_printable_lazy(
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
        Ok(ItsOutgoingInterchainTransfer {
            inner_message,
            destination_chain,
        })
    }
}

impl<N: Network> TryFrom<&OutgoingInterchainTransfer<N>> for InterchainTransfer {
    type Error = Report<Error>;

    fn try_from(outgoing_transfer: &OutgoingInterchainTransfer<N>) -> Result<Self, Self::Error> {
        let its_token_id = ItsTokenIdNewType(outgoing_transfer.its_token_id);
        let source_address =
            StringEncoder::encode_string(&outgoing_transfer.source_address.to_string())
                .map_err(Error::from)?
                .decode()
                .map_err(Error::from)?
                .into_bytes()
                .try_into()
                .map_err(Error::from)?;

        let destination_address = {
            let destination_address =
                StringEncoder::from_slice(&outgoing_transfer.destination_address)
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
            amount: cosmwasm_std::Uint256::from_u128(outgoing_transfer.amount)
                .try_into()
                .map_err(Error::from)?,
            data: None,
        })
    }
}

impl<N: Network> TryFrom<ItsOutgoingInterchainTransfer<N>> for HubMessage {
    type Error = Report<Error>;

    fn try_from(outgoing_transfer: ItsOutgoingInterchainTransfer<N>) -> Result<Self, Self::Error> {
        let interchain_transfer = InterchainTransfer::try_from(&outgoing_transfer.inner_message)
            .attach_printable_lazy(|| format!("{:#?}", outgoing_transfer.inner_message))?;

        let destination_chain: SafeGmpChainName =
            SafeGmpChainName::new(outgoing_transfer.destination_chain).change_context(
                Error::TranslationFailed("Failed to translate chain name".to_string()),
            )?;

        Ok(HubMessage::SendToHub {
            destination_chain: ChainNameRaw::try_from(destination_chain).map_err(Error::from)?,
            message: Message::InterchainTransfer(interchain_transfer),
        })
    }
}
