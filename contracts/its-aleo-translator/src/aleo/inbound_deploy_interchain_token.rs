use std::str::FromStr as _;

use aleo_string_encoder::StringEncoder;
use error_stack::Report;
use snarkvm_cosmwasm::console::program::Network;
use snarkvm_cosmwasm::console::types::Address;

use super::token_id_conversion::ItsTokenIdNewType;
use super::Error;

use super::generated::FromRemoteDeployInterchainToken;

pub(crate) fn convert_incoming_deploy_interchain_token<N: Network>(
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
