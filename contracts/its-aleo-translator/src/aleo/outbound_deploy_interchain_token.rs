use aleo_gmp_types::SafeGmpChainName;
use aleo_string_encoder::StringEncoder;
use axelar_wasm_std::nonempty;
use error_stack::{Report, ResultExt};
use interchain_token_service_std::{
    DeployInterchainToken as DeployInterchainTokenItsHub, HubMessage, Message, TokenId,
};
use router_api::ChainNameRaw;
use snarkvm_cosmwasm::prelude::Network;

use super::generated::RemoteDeployInterchainToken;
use super::token_id_conversion::ItsTokenIdNewType;
use super::Error;

pub(crate) fn convert_outgoing_deploy<N: Network>(
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
