//! Verification implementation of Starknet JSON RPC client's verification of
//! transaction existence

use std::str::FromStr;

use async_trait::async_trait;
use mockall::{automock, mock};
use starknet_core::types::{
    ExecutionResult, FieldElement, MaybePendingTransactionReceipt, TransactionReceipt,
};
use starknet_providers::jsonrpc::{HttpTransport, HttpTransportError, JsonRpcTransport};
use starknet_providers::{JsonRpcClient, Provider, ProviderError};
use thiserror::Error;

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
pub struct Client<T>
where
    T: JsonRpcTransport + Send + Sync + 'static,
{
    client: JsonRpcClient<T>,
}

#[automock]
#[async_trait]
pub trait StarknetClient<T>
where
    T: JsonRpcTransport + Send + Sync + 'static,
{
    fn new(transport: T) -> Result<Client<T>, StarknetClientError>;
    async fn get_event_by_hash(
        &self,
        tx_hash: &str,
    ) -> Result<Option<(String, ContractCallEvent)>, StarknetClientError>;
}

#[async_trait]
impl<T> StarknetClient<T> for Client<T>
where
    T: JsonRpcTransport + Send + Sync + 'static,
{
    /// Constructor.
    /// Expects URL of any JSON RPC entry point of Starknet, which you can find
    /// as constants in the `networks.rs` module
    fn new(transport: T) -> Result<Self, StarknetClientError> {
        Ok(Client {
            client: JsonRpcClient::new(transport),
        })
    }

    /// Using given transaction hash, tries to fetch it from given
    /// `starknet_url`. Returns error if request fails, `false` if internal
    /// error returned by querry and `true` if transaction found
    async fn get_event_by_hash(
        &self,
        tx_hash: &str,
    ) -> Result<Option<(String, ContractCallEvent)>, StarknetClientError> {
        println!("TEST HASH {}", tx_hash);
        // TODO: Check ACCEPTED ON L1 times and decide if we should use it
        //
        // Finality status is always at least ACCEPTED_ON_L2 and this is what we're
        // looking for, because ACCEPTED_ON_L1 (Ethereum) will take a very long time.
        let receipt_type = self
            .client
            .get_transaction_receipt(FieldElement::from_str(tx_hash.as_ref()).unwrap())
            .await
            .map_err(StarknetClientError::FetchingReceipt)?;

        dbg!(receipt_type.clone());
        if *receipt_type.execution_result() != ExecutionResult::Succeeded {
            return Err(StarknetClientError::UnsuccessfulTx);
        }

        let event: Option<(String, ContractCallEvent)> = match receipt_type {
            // TODO: There is also a PendingReceipt type. Should we handle it?
            MaybePendingTransactionReceipt::Receipt(receipt) => match receipt {
                TransactionReceipt::Invoke(tx) => {
                    dbg!(tx.events.clone());
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

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use axum::async_trait;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use starknet_core::types::{
        ExecutionResources, ExecutionResult, FeePayment, FieldElement, FromStrError,
        InvokeTransactionReceipt, MaybePendingTransactionReceipt, MsgToL1, PriceUnit,
        TransactionFinalityStatus, TransactionReceipt,
    };
    use starknet_core::utils::starknet_keccak;
    use starknet_providers::jsonrpc::{
        HttpTransportError, JsonRpcMethod, JsonRpcResponse, JsonRpcTransport,
    };
    use starknet_providers::{JsonRpcClient, Provider, ProviderError};

    use super::{Client, StarknetClient};

    fn get_valid_contract_call_event() -> starknet_core::types::Event {
        let data: Result<Vec<FieldElement>, FromStrError> = vec![
            "0xb3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca",
            "0x0000000000000000000000000000000000000000000000000000000000000000", // 0 datas
            "0x00000000000000000000000000000000000000000000000000000068656c6c6f", // "hello"
            "0x0000000000000000000000000000000000000000000000000000000000000005", // 5 bytes
            "0x0000000000000000000000000000000056d9517b9c948127319a09a7a36deac8", // keccak256(hello)
            "0x000000000000000000000000000000001c8aff950685c2ed4bc3174f3472287b",
            "0x0000000000000000000000000000000000000000000000000000000000000005", // 5 bytes
            "0x0000000000000000000000000000000000000000000000000000000000000068", // h
            "0x0000000000000000000000000000000000000000000000000000000000000065", // e
            "0x000000000000000000000000000000000000000000000000000000000000006c", // l
            "0x000000000000000000000000000000000000000000000000000000000000006c", // l
            "0x000000000000000000000000000000000000000000000000000000000000006f", // o
        ]
        .into_iter()
        .map(FieldElement::from_str)
        .collect();

        starknet_core::types::Event {
            // I think it's a pedersen hash, but  we don't use it, so any value should do
            from_address: starknet_keccak("some_from_address".as_bytes()),
            keys: vec![
                starknet_keccak("ContractCall".as_bytes()),
                // destination_chain is the second key
                FieldElement::from_str(
                    "0x00000000000000000000000000000064657374696e6174696f6e5f636861696e",
                )
                .unwrap(),
            ],
            data: data.unwrap(),
        }
    }

    struct MockTransport;

    #[async_trait]
    impl JsonRpcTransport for MockTransport {
        type Error = HttpTransportError;

        async fn send_request<P, R>(
            &self,
            method: JsonRpcMethod,
            params: P,
        ) -> Result<JsonRpcResponse<R>, Self::Error>
        where
            P: Serialize + Send + Sync,
            R: DeserializeOwned,
        {
            let response_mock = "{
  \"jsonrpc\": \"2.0\",
  \"result\": {
    \"type\": \"INVOKE\",
    \"transaction_hash\": \"0x11fda9f99ec826c5b865be0a982014b208b3958b99c9b44896f762d6eabd023\",
    \"actual_fee\": {
      \"amount\": \"0x3062e4c46d4\",
      \"unit\": \"WEI\"
    },
    \"execution_status\": \"SUCCEEDED\",
    \"finality_status\": \"ACCEPTED_ON_L2\",
    \"block_hash\": \"0x5820e3a0aaceebdbda0b308fdf666eff64f263f6ed8ee74d6f78683b65a997b\",
    \"block_number\": 637493,
    \"messages_sent\": [],
    \"events\": [
      {
        \"from_address\": \"0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d\",
        \"keys\": [
          \"0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9\"
        ],
        \"data\": [
          \"0x3ccba12965a96dd6470b11a0b3c1c3ff12bc107cedbe3b03aaf92424c550995\",
          \"0x4505a9f06f2bd639b6601f37a4dc0908bb70e8e0e0c34b1220827d64f4fc066\",
          \"0x10f0b8d1f64fb960000\",
          \"0x0\"
        ]
      }
    ],
    \"execution_resources\": {
      \"steps\": 137449,
      \"pedersen_builtin_applications\": 241,
      \"range_check_builtin_applications\": 9402,
      \"bitwise_builtin_applications\": 143,
      \"ec_op_builtin_applications\": 3
    }
  },
  \"id\": 0
}";
            let parsed_response = serde_json::from_str(response_mock).map_err(Self::Error::Json)?;

            Ok(parsed_response)
        }
    }

    #[tokio::test]
    async fn existing_tx_hash() {
        let mock_client = Client::new(MockTransport).unwrap();
        mock_client
            .get_event_by_hash(FieldElement::ONE.to_string().as_str())
            .await
            .unwrap();
    }
}

// struct MockStarknetClient;
//
// impl Provider for MockStarknetClient {}
//
// struct MockJsonRpcClient;
// impl MockJsonRpcClient {
//     fn get_transaction_receipt(
//         tx_hash: FieldElement,
//     ) -> Result<MaybePendingTransactionReceipt, ProviderError> {
//         Ok(MaybePendingTransactionReceipt::Receipt(
//             TransactionReceipt::Invoke(InvokeTransactionReceipt {
//                 transaction_hash: tx_hash,
//                 actual_fee: FeePayment {
//                     amount: FieldElement::ONE,
//                     unit: PriceUnit::Wei,
//                 },
//                 finality_status: TransactionFinalityStatus::AcceptedOnL1,
//                 block_hash: FieldElement::ONE,
//                 block_number: 1,
//                 messages_sent: vec![MsgToL1 {
//                     from_address: FieldElement::ONE,
//                     to_address: FieldElement::ONE,
//                     payload: vec![FieldElement::ONE],
//                 }],
//                 events: vec![get_valid_contract_call_event()],
//                 execution_resources: ExecutionResources {
//                     steps: 1,
//                     memory_holes: None,
//                     range_check_builtin_applications: None,
//                     pedersen_builtin_applications: None,
//                     poseidon_builtin_applications: None,
//                     ec_op_builtin_applications: None,
//                     ecdsa_builtin_applications: None,
//                     bitwise_builtin_applications: None,
//                     keccak_builtin_applications: None,
//                     segment_arena_builtin: None,
//                 },
//                 execution_result: ExecutionResult::Succeeded,
//             }),
//         ))
//     }
// }
//
// fn get_mock_starknet_client() -> MockStarknetClient {
//     MockStarknetClient {
//         client: MockJsonRpcClient {},
//     }
// }
