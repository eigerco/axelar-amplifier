use std::str::FromStr;

use aleo_gateway_types::{FromRemoteDeployInterchainToken, IncomingInterchainTransfer};
use aleo_string_encoder::StringEncoder;
use cosmwasm_std::Uint128;
use snarkvm_cosmwasm::prelude::{Address, Field, Network, Plaintext};

use crate::error::Error;
use crate::token_id_conversion::ItsTokenIdNewType;

/// This trait provides a way to convert Rust structs to structs that
/// can be converted to Plaintext for the Aleo network.
pub trait AxelarToLeo<N: Network> {
    type LeoType: for<'a> TryFrom<&'a Plaintext<N>>;
    type Error;

    fn to_leo(&self) -> Result<Self::LeoType, Self::Error>;
}

// ITS types
impl<N: Network> AxelarToLeo<N> for interchain_token_service_std::InterchainTransfer {
    type LeoType = IncomingInterchainTransfer<N>;
    type Error = Error;

    fn to_leo(&self) -> Result<Self::LeoType, Self::Error> {
        let its_token_id = ItsTokenIdNewType::from(self.token_id);

        let source_address =
            StringEncoder::encode_bytes(self.source_address.as_slice())?.to_array()?;

        let destination_address =
            Address::from_str(std::str::from_utf8(&self.destination_address)?)?;

        let amount = Uint128::try_from(*self.amount)?.u128();

        Ok(IncomingInterchainTransfer {
            its_token_id: *its_token_id,
            source_address,
            destination_address,
            amount,
        })
    }
}

impl<N: Network> AxelarToLeo<N> for interchain_token_service_std::DeployInterchainToken {
    type LeoType = FromRemoteDeployInterchainToken<N>;
    type Error = Error;

    fn to_leo(&self) -> Result<Self::LeoType, Self::Error> {
        let its_token_id = ItsTokenIdNewType::from(self.token_id);

        let name: [u128; 2] = StringEncoder::encode_string(&self.name)?.to_array()?;

        let symbol: [u128; 2] = StringEncoder::encode_string(&self.symbol)?.to_array()?;

        let minter = match &self.minter {
            Some(hex) => Address::from_str(std::str::from_utf8(hex)?)?,
            None => Address::zero(),
        };

        Ok(FromRemoteDeployInterchainToken {
            its_token_id: *its_token_id,
            name: name[0],
            symbol: symbol[0],
            decimals: self.decimals,
            minter,
        })
    }
}

impl<N: Network> AxelarToLeo<N> for interchain_token_service_std::LinkToken {
    type LeoType = aleo_gateway_types::ReceivedLinkToken<N>;
    type Error = Error;

    fn to_leo(&self) -> Result<Self::LeoType, Self::Error> {
        let its_token_id = ItsTokenIdNewType::from(self.token_id);

        let source_token_address =
            StringEncoder::encode_bytes(self.source_token_address.as_slice())?.to_array()?;

        let destination_token_address: Field<N> =
            Field::from_str(std::str::from_utf8(&self.destination_token_address)?)?;

        let operator = match &self.params {
            Some(hex) => Address::from_str(std::str::from_utf8(hex)?)
                .map_err(|_| Error::InvalidOperatorAddress)?,
            None => Address::zero(),
        };

        Ok(aleo_gateway_types::ReceivedLinkToken {
            its_token_id: *its_token_id,
            token_manager_type: self
                .token_manager_type
                .to_string()
                .parse()
                .map_err(|_| Error::InvalidTokenManagerType)?,
            source_token_address,
            destination_token_address,
            operator,
        })
    }
}
