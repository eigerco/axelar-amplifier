use aleo_gmp_types::SafeGmpChainName;
use aleo_string_encoder::StringEncoder;
use axelar_wasm_std::nonempty;
use cosmwasm_std::Uint128;
use error_stack::{Report, ResultExt};
use interchain_token_service_std::{
    DeployInterchainToken as DeployInterchainTokenItsHub, HubMessage,
    InterchainTransfer as InterchainTransferItsHub, Message, TokenId,
};
use router_api::ChainNameRaw;
use snarkvm_cosmwasm::prelude::{Address, Network};
use std::str::FromStr as _;

use crate::aleo::token_id_conversion::ItsTokenIdNewType;
use crate::aleo::Error;

use super::generated::{
    FromRemoteDeployInterchainToken, IncomingInterchainTransfer, ItsOutgoingInterchainTransfer,
    RemoteDeployInterchainToken,
};

pub fn incoming_deploy_interchain_token<N: Network>(
    deploy_message: interchain_token_service_std::DeployInterchainToken,
) -> Result<FromRemoteDeployInterchainToken<N>, Report<Error>> {
    let its_token_id = ItsTokenIdNewType::from(deploy_message.token_id);

    let name: [u128; 2] = StringEncoder::encode_string(&deploy_message.name)
        .map_err(Error::from)?
        .to_array()
        .map_err(Error::from)?;

    let symbol: [u128; 2] = StringEncoder::encode_string(&deploy_message.symbol)
        .map_err(Error::from)?
        .to_array()
        .map_err(Error::from)?;

    let minter = match deploy_message.minter {
        Some(hex) => Address::<N>::from_str(std::str::from_utf8(&hex).map_err(Error::from)?)
            .map_err(Error::from)?,
        None => Address::<N>::zero(),
    };

    Ok(FromRemoteDeployInterchainToken {
        its_token_id: its_token_id.0,
        name: name[0],
        symbol: symbol[0],
        decimals: deploy_message.decimals,
        minter,
    })
}

pub fn incoming_transfer<N: Network>(
    transfer: &InterchainTransferItsHub,
) -> Result<IncomingInterchainTransfer<N>, Report<Error>> {
    Ok(IncomingInterchainTransfer {
        its_token_id: ItsTokenIdNewType::from(transfer.token_id).0,
        source_address: StringEncoder::encode_bytes(transfer.source_address.as_slice())
            .map_err(Error::from)?
            .to_array()
            .map_err(Error::from)?,
        destination_address: Address::from_str(
            std::str::from_utf8(&transfer.destination_address).map_err(Error::from)?,
        )
        .map_err(Error::from)?,
        amount: Uint128::try_from(*transfer.amount)
            .map_err(Error::from)
            .change_context_lazy(|| {
                Error::TranslationFailed(format!(
                    "Failed to convert amount to Uint128, amount = {}",
                    transfer.amount
                ))
            })?
            .u128(),
    })
}

pub fn outgoing_deploy<N: Network>(
    remote_deploy: RemoteDeployInterchainToken,
) -> Result<HubMessage, Report<Error>> {
    let deploy_interchain_token = remote_deploy.payload;

    let deploy_interchain_token_its_hub = DeployInterchainTokenItsHub {
        token_id: TokenId::from(ItsTokenIdNewType(deploy_interchain_token.its_token_id)),
        name: axelar_wasm_std::nonempty::String::try_from(
            StringEncoder::from_slice(&[deploy_interchain_token.name])
                .decode()
                .map_err(Error::from)?,
        )
        .map_err(Error::from)?,
        symbol: axelar_wasm_std::nonempty::String::try_from(
            StringEncoder::from_slice(&[deploy_interchain_token.symbol])
                .decode()
                .map_err(Error::from)?,
        )
        .map_err(Error::from)?,
        decimals: deploy_interchain_token.decimals,
        minter: {
            let minter = StringEncoder::from_slice(&deploy_interchain_token.minter)
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
        },
    };

    let destination_chain = SafeGmpChainName::new(remote_deploy.destination_chain)
        .change_context_lazy(|| {
            Error::TranslationFailed(
                "Failed to create SafeGmpChainName from destination_chain".to_string(),
            )
        })?;

    Ok(HubMessage::SendToHub {
        destination_chain: ChainNameRaw::try_from(destination_chain).map_err(Error::from)?,
        message: Message::DeployInterchainToken(deploy_interchain_token_its_hub),
    })
}

pub fn outgoing_transfer<N: Network>(
    outgoing_transfer: ItsOutgoingInterchainTransfer<N>,
) -> Result<HubMessage, Report<Error>> {
    let interchain_transfer = InterchainTransferItsHub {
        token_id: TokenId::from(ItsTokenIdNewType(
            outgoing_transfer.inner_message.its_token_id,
        )),
        source_address: StringEncoder::encode_string(
            &outgoing_transfer.inner_message.source_address.to_string(),
        )
        .map_err(Error::from)?
        .decode()
        .map_err(Error::from)?
        .into_bytes()
        .try_into()
        .map_err(Error::from)?,
        destination_address: {
            let destination_address =
                StringEncoder::from_slice(&outgoing_transfer.inner_message.destination_address)
                    .decode()
                    .map_err(Error::from)?;

            let destination_address = destination_address
                .strip_prefix("0x")
                .unwrap_or(&destination_address);

            nonempty::HexBinary::try_from(hex::decode(destination_address).map_err(Error::from)?)
                .map_err(Error::from)?
        },
        amount: cosmwasm_std::Uint256::from_u128(outgoing_transfer.inner_message.amount)
            .try_into()
            .map_err(Error::from)
            .change_context_lazy(|| {
                Error::TranslationFailed(format!(
                    "Failed to convert amount to Uint256, amount = {}",
                    outgoing_transfer.inner_message.amount
                ))
            })?,
        data: None,
    };

    let destination_chain: SafeGmpChainName =
        SafeGmpChainName::new(outgoing_transfer.destination_chain).change_context_lazy(|| {
            Error::TranslationFailed("Failed to translate chain name".to_string())
        })?;

    Ok(HubMessage::SendToHub {
        destination_chain: ChainNameRaw::try_from(destination_chain).map_err(Error::from)?,
        message: Message::InterchainTransfer(interchain_transfer),
    })
}
