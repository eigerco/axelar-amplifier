use std::str::FromStr as _;

use aleo_gateway::types::{GmpAddress, GmpChainName, ItsTokenId};
use aleo_string_encoder::StringEncoder;
use cosmwasm_std::Uint128;
use error_stack::Report;
use interchain_token_service_std::InterchainTransfer;
use plaintext_trait::ToPlaintext;
use snarkvm_cosmwasm::console::program::Network;
use snarkvm_cosmwasm::console::types::Address;

use super::token_id_conversion::ItsTokenIdNewType;
use super::Error;

/// Wrapper for interchain transfer messages that includes source chain information.
///
/// This struct represents the [HubMessage::ReceiveFromHub](interchain_token_service_std::primitives::HubMessage::ReceiveFromHub)
/// that contains [InterchainTransfer] message type.
#[derive(ToPlaintext, Clone, Debug)]
pub struct ItsInboundInterchainTransfer<N: Network> {
    /// The inner interchain transfer message containing the transfer details.
    pub inner_message: InboundInterchainTransfer<N>,
    /// The source chain from which the interchain transfer originated.
    pub source_chain: GmpChainName,
}

/// Represents an inbound interchain transfer message received from a remote chain.
///
/// This struct contains the essential information needed to process an interchain transfer
/// on the Aleo network.
#[derive(ToPlaintext, Clone, Copy, Debug)]
pub struct InboundInterchainTransfer<N: Network> {
    /// Unique identifier for the interchain token
    pub its_token_id: ItsTokenId,
    /// Source address from which the transfer originated, encoded as a GmpAddress.
    pub source_address: GmpAddress,
    /// Destination address on the Aleo network where the transfer is directed.
    pub destination_address: Address<N>,
    /// Amount of tokens being transferred, represented as a u128.
    pub amount: u128,
}

impl<N: Network> TryFrom<&InterchainTransfer> for InboundInterchainTransfer<N> {
    type Error = Report<Error>;

    fn try_from(transfer: &InterchainTransfer) -> Result<Self, Self::Error> {
        let its_token_id = ItsTokenIdNewType::from(transfer.token_id).0;

        let source_address = StringEncoder::encode_bytes(transfer.source_address.as_slice())
            .map_err(Error::from)?
            .to_array()
            .map_err(Error::from)?;

        let destination_address = Address::from_str(
            std::str::from_utf8(&transfer.destination_address).map_err(Error::from)?,
        )
        .map_err(Error::from)?;

        let amount = Uint128::try_from(*transfer.amount)
            .map_err(Error::from)?
            .u128();

        Ok(InboundInterchainTransfer {
            its_token_id,
            source_address,
            destination_address,
            amount,
        })
    }
}
