//! Verification implementation of Starknet JSON RPC client's verification of
//! transaction existence

use std::str::FromStr;

use starknet_core::types::{ExecutionResult, FieldElement, MaybePendingTransactionReceipt};
use starknet_core::utils::parse_cairo_short_string;
use starknet_providers::jsonrpc::HttpTransport;
use starknet_providers::{JsonRpcClient, Provider, ProviderError};
use thiserror::Error;
use url::Url;

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
    /// Expects URL of any JSON RPC entry point of Starknet.
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
    ) -> Result<Option<starknet_core::types::Event>, StarknetClientError> {
        println!(
            "TX_IDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD {:?}",
            tx_hash.as_ref()
        );

        // TODO: Check ACCEPTED ON L1 times and decide if we should use it
        //
        // Finality status is always at least ACCEPTED_ON_L2 and this is what we're
        // looking for, because ACCEPTED_ON_L1 (Ethereum) will take a very long time.
        let receipt_type = self
            .client
            // .get_transaction_receipt(FieldElement::from_str(tx_hash.as_ref()).unwrap())
            .get_transaction_receipt(FieldElement::from_str(tx_hash.as_ref()).unwrap())
            .await
            .map_err(StarknetClientError::FetchingReceipt)?;

        println!(
            "KOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOORRRRR {:#?}",
            receipt_type
        );

        if *receipt_type.execution_result() != ExecutionResult::Succeeded {
            return Err(StarknetClientError::UnsuccessfulTx);
        }

        let event: Option<starknet_core::types::Event> = match receipt_type {
            MaybePendingTransactionReceipt::Receipt(receipt) => match receipt {
                starknet_core::types::TransactionReceipt::Invoke(tx) => {
                    // There should be only one event with key "starknet"

                    for e in tx.clone().events {
                        for d in e.data {
                            println!("EVENT DATA -> {:?}", d);
                            println!("EVENT DATA -> {:?}", d.to_string());
                            println!("EVENT DATA -> {:?}", d.to_bytes_be());
                            println!("EVENT DATA -> {:?}", parse_cairo_short_string(&d));
                            println!("");
                        }

                        for k in e.keys {
                            println!("EVENT KEY -> {:?}", k);
                            println!("EVENT KEY -> {:?}", k.to_string());
                            println!("EVENT KEY -> {:?}", k.to_bytes_be());
                            println!("EVENT KEY -> {:?}", parse_cairo_short_string(&k));
                            println!("");
                        }
                    }

                    let event: Option<starknet_core::types::Event> = tx
                        .events
                        .into_iter()
                        .find(|event| event.keys.iter().any(|key| key.to_string() == "starknet"));

                    println!("EVENTTTTTTTTTTTTTTTTTTTTT {:?}", event);

                    event
                }
                starknet_core::types::TransactionReceipt::L1Handler(_) => None,
                starknet_core::types::TransactionReceipt::Declare(_) => None,
                starknet_core::types::TransactionReceipt::Deploy(_) => None,
                starknet_core::types::TransactionReceipt::DeployAccount(_) => None,
            },
            // TODO: Not sure if we should handle pending transactions?
            MaybePendingTransactionReceipt::PendingReceipt(_) => None,
        };

        Ok(event)
    }
}
