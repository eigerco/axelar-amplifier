use std::str::FromStr as _;

use aleo_gateway::types::{GmpChainName, ItsTokenId};
use aleo_string_encoder::StringEncoder;
use error_stack::Report;
use plaintext_trait::ToPlaintext;
use snarkvm_cosmwasm::console::{program::Network, types::Address};

use super::token_id_conversion::ItsTokenIdNewType;
use super::Error;

/// Represents a deploy interchain token message received from a remote chain.
///
/// This struct contains the essential information needed to deploy an interchain token
/// on the Aleo network, including token metadata and minter authorization.
///
/// Note: Axelar supports 20 characters for the token name, but on Aleo we can have only upto 16 characters.
/// the last 4 characters are ignored.
#[derive(ToPlaintext, Clone, Copy, Debug)]
pub struct FromRemoteDeployInterchainToken<N: Network> {
    /// Unique identifier for the interchain token
    pub its_token_id: ItsTokenId,
    /// Token name encoded as a u128 value
    pub name: u128,
    /// Token symbol encoded as a u128 value
    pub symbol: u128,
    /// Number of decimal places for the token
    pub decimals: u8,
    /// Address authorized to mint tokens (zero address if no minter)
    pub minter: Address<N>,
}

/// Wrapper for deploy interchain token messages that includes source chain information.
///
/// This struct represents the HubMessage::ReceiveFromHub
#[derive(ToPlaintext, Clone, Copy, Debug)]
pub struct ItsMessageDeployInterchainToken<N: Network> {
    pub inner_message: FromRemoteDeployInterchainToken<N>,
    pub source_chain: GmpChainName,
}

impl<N: Network> TryFrom<interchain_token_service_std::DeployInterchainToken>
    for FromRemoteDeployInterchainToken<N>
{
    type Error = Report<Error>;

    /// Converts a standard DeployInterchainToken message into an Aleo-compatible format.
    ///
    /// This conversion handles:
    /// - Token ID conversion using ItsTokenIdNewType wrapper
    /// - String encoding for name and symbol (taking first u128 of encoded array)
    /// - Minter address parsing from hex bytes (defaults to zero address if None)
    ///
    /// # Arguments
    /// * `deploy_message` - The source deployment message to convert
    ///
    /// # Returns
    /// * `Ok(FromRemoteDeployInterchainToken)` - Successfully converted message
    /// * `Err(Error)` - Conversion failed due to encoding or parsing errors
    ///
    /// # Errors
    /// - String encoding failures for name or symbol
    /// - Invalid UTF-8 in minter hex string
    /// - Invalid address format in minter field
    fn try_from(
        deploy_message: interchain_token_service_std::DeployInterchainToken,
    ) -> Result<Self, Self::Error> {
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
}
