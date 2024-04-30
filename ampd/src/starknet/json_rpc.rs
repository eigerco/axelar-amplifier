//! Verification implementation of Starknet JSON RPC client's verification of
//! transaction existence

use std::str::FromStr;

use starknet_core::types::{
    ExecutionResult, FieldElement, MaybePendingTransactionReceipt, TransactionReceipt,
};
use starknet_core::utils::parse_cairo_short_string;
use starknet_providers::jsonrpc::HttpTransport;
use starknet_providers::{JsonRpcClient, Provider, ProviderError};
use thiserror::Error;
use url::Url;

use crate::starknet::events::contract_call::ContractCallEvent;

#[derive(Debug, Error)]
pub enum StarknetClientError {
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    JsonDeserializeError(#[from] serde_json::Error),
    #[error("Failed to fetch tx receipt: {0}")]
    FetchingReceipt(#[from] ProviderError),
    #[error("Tx not successful")]
    UnsuccessfulTx,
}

/// Implementor of verification method(s) for given network using JSON RPC
/// client.
pub struct StarknetClient {
    client: JsonRpcClient<HttpTransport>,
}

impl StarknetClient {
    /// Constructor.
    /// Expects URL of any JSON RPC entry point of Starknet, which you can find
    /// as constants in the `networks.rs` module
    pub fn new(starknet_url: impl AsRef<str>) -> Result<Self, StarknetClientError> {
        Ok(StarknetClient {
            client: JsonRpcClient::new(HttpTransport::new(Url::parse(starknet_url.as_ref())?)),
        })
    }

    /// Using given transaction hash, tries to fetch it from given
    /// `starknet_url`. Returns error if request fails, `false` if internal
    /// error returned by querry and `true` if transaction found
    pub async fn get_event_by_hash(
        &self,
        tx_hash: impl AsRef<str>,
    ) -> Result<Option<(String, ContractCallEvent)>, StarknetClientError> {
        // TODO: Check ACCEPTED ON L1 times and decide if we should use it
        //
        // Finality status is always at least ACCEPTED_ON_L2 and this is what we're
        // looking for, because ACCEPTED_ON_L1 (Ethereum) will take a very long time.
        let receipt_type = self
            .client
            .get_transaction_receipt(FieldElement::from_str(tx_hash.as_ref()).unwrap())
            .await
            .map_err(StarknetClientError::FetchingReceipt)?;

        if *receipt_type.execution_result() != ExecutionResult::Succeeded {
            return Err(StarknetClientError::UnsuccessfulTx);
        }

        let event: Option<(String, ContractCallEvent)> = match receipt_type {
            MaybePendingTransactionReceipt::Receipt(receipt) => match receipt {
                TransactionReceipt::Invoke(tx) => {
                    // There should be only one ContractCall event per gateway tx
                    tx.events
                        .iter()
                        .filter_map(|e| {
                            if let Ok(cce) = ContractCallEvent::try_from(e.clone()) {
                                Some((format!("0x{:064x}", tx.transaction_hash).to_owned(), cce))
                            } else {
                                None
                            }
                        })
                        .next()
                }
                TransactionReceipt::L1Handler(_) => None,
                TransactionReceipt::Declare(_) => None,
                TransactionReceipt::Deploy(_) => None,
                TransactionReceipt::DeployAccount(_) => None,
            },
            MaybePendingTransactionReceipt::PendingReceipt(_) => None,
        };

        Ok(event)
    }
}
