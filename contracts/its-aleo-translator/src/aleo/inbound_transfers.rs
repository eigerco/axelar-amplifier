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

#[derive(ToPlaintext, Clone, Debug)]
pub struct ItsIncomingInterchainTransfer<N: Network> {
    pub inner_message: IncomingInterchainTransfer<N>,
    pub source_chain: GmpChainName,
}

#[derive(ToPlaintext, Clone, Copy, Debug)]
pub struct IncomingInterchainTransfer<N: Network> {
    pub its_token_id: ItsTokenId,
    pub source_address: GmpAddress,
    pub destination_address: Address<N>,
    pub amount: u128,
}

impl<N: Network> TryFrom<&InterchainTransfer> for IncomingInterchainTransfer<N> {
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

        Ok(IncomingInterchainTransfer {
            its_token_id,
            source_address,
            destination_address,
            amount,
        })
    }
}
