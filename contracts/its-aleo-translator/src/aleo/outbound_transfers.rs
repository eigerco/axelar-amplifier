use aleo_gmp_types::SafeGmpChainName;
use aleo_string_encoder::StringEncoder;
use axelar_wasm_std::nonempty;
use error_stack::{Report, ResultExt};
use interchain_token_service_std::{
    HubMessage, InterchainTransfer as InterchainTransferItsHub, Message, TokenId,
};
use router_api::ChainNameRaw;
use snarkvm_cosmwasm::prelude::Network;

use crate::aleo::token_id_conversion::ItsTokenIdNewType;
use crate::aleo::Error;

use super::generated::ItsOutgoingInterchainTransfer;

pub(crate) fn convert_outgoing_transfer<N: Network>(
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
            .map_err(Error::from)?,
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
