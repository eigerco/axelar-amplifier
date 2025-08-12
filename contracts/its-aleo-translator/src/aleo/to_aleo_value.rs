use std::str::FromStr as _;

use aleo_gmp_types::SafeGmpChainName;
use aleo_string_encoder::StringEncoder;
use cosmwasm_std::Uint128;
use error_stack::{Report, ResultExt};
use interchain_token_service_std::InterchainTransfer as InterchainTransferItsHub;
use router_api::ChainNameRaw;
use snarkvm_cosmwasm::prelude::{Address, Network, Value};

use crate::aleo::generated::{FromRemoteDeployInterchainToken, IncomingInterchainTransfer};
use crate::aleo::token_id_conversion::ItsTokenIdNewType;
use crate::aleo::Error;

/// Trait to convert a message to an Aleo Value
pub trait ToAleoValue<N: Network> {
    type Error;

    fn to_aleo_value(self, source_chain: ChainNameRaw) -> Result<Value<N>, Self::Error>;
}

impl<N: Network> ToAleoValue<N> for interchain_token_service_std::DeployInterchainToken {
    type Error = Report<Error>;

    fn to_aleo_value(self, source_chain: ChainNameRaw) -> Result<Value<N>, Self::Error> {
        let its_token_id = ItsTokenIdNewType::from(self.token_id);

        let name: [u128; 2] = StringEncoder::encode_string(&self.name)
            .map_err(Error::from)?
            .to_array()
            .map_err(Error::from)?;

        let symbol: [u128; 2] = StringEncoder::encode_string(&self.symbol)
            .map_err(Error::from)?
            .to_array()
            .map_err(Error::from)?;

        let minter = match self.minter {
            Some(hex) => Address::<N>::from_str(std::str::from_utf8(&hex).map_err(Error::from)?)
                .map_err(Error::from)?,
            None => Address::<N>::zero(),
        };

        let source_chain = SafeGmpChainName::try_from(&source_chain).change_context_lazy(|| {
            Error::TranslationFailed(format!(
                "Failed to translate chain name, '{source_chain:?}'"
            ))
        })?;

        let deploy_interchain_token = FromRemoteDeployInterchainToken {
            its_token_id: *its_token_id,
            name: name[0],
            symbol: symbol[0],
            decimals: self.decimals,
            minter,
        };

        let message = super::generated::ItsMessageDeployInterchainToken::<N> {
            inner_message: deploy_interchain_token,
            source_chain: source_chain.chain_name(),
        };

        let aleo_plaintext =
            snarkvm_cosmwasm::prelude::Plaintext::try_from(&message).map_err(|e| {
                Error::TranslationFailed(format!(
                    "Failed to convert deploy interchain token to Aleo plaintext: {e}"
                ))
            })?;

        Ok(snarkvm_cosmwasm::prelude::Value::Plaintext(aleo_plaintext))
    }
}

impl<N: Network> ToAleoValue<N> for InterchainTransferItsHub {
    type Error = Report<Error>;

    fn to_aleo_value(self, source_chain: ChainNameRaw) -> Result<Value<N>, Self::Error> {
        let its_token_id = ItsTokenIdNewType::from(self.token_id);

        let source_address = StringEncoder::encode_bytes(self.source_address.as_slice())
            .map_err(Error::from)?
            .to_array()
            .map_err(Error::from)?;

        let destination_address = Address::<N>::from_str(
            std::str::from_utf8(&self.destination_address).map_err(Error::from)?,
        )
        .map_err(Error::from)?;

        let amount = Uint128::try_from(*self.amount)
            .map_err(Error::from)
            .change_context_lazy(|| {
                Error::TranslationFailed(format!(
                    "Failed to convert amount to Uint128, amount = {}",
                    self.amount
                ))
            })?
            .u128();

        let source_chain = SafeGmpChainName::try_from(&source_chain).change_context_lazy(|| {
            Error::TranslationFailed(format!(
                "Failed to translate chain name, '{source_chain:?}'"
            ))
        })?;

        let interchain_transfer = IncomingInterchainTransfer {
            its_token_id: *its_token_id,
            source_address,
            destination_address,
            amount,
        };

        let message = super::generated::ItsIncomingInterchainTransfer::<N> {
            inner_message: interchain_transfer,
            source_chain: source_chain.chain_name(),
        };

        let aleo_plaintext =
            snarkvm_cosmwasm::prelude::Plaintext::try_from(&message).map_err(|e| {
                Error::TranslationFailed(format!(
                    "Failed to convert interchain transfer to Aleo plaintext: {e}"
                ))
            })?;

        Ok(snarkvm_cosmwasm::prelude::Value::Plaintext(aleo_plaintext))
    }
}
