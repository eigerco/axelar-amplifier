use std::str::FromStr as _;

use aleo_string_encoder::StringEncoder;
use cosmwasm_std::Uint128;
use error_stack::Report;
use interchain_token_service_std::InterchainTransfer;
use snarkvm_cosmwasm::console::program::Network;
use snarkvm_cosmwasm::console::types::Address;

use super::token_id_conversion::ItsTokenIdNewType;
use super::Error;

use super::generated::IncomingInterchainTransfer;

pub fn convert_incoming_transfer<N: Network>(
    transfer: &InterchainTransfer,
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
            .map_err(Error::from)?
            .u128(),
    })
}
