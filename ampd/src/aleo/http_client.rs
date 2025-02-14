use std::str::FromStr;

use aleo_types::address::Address;
use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use aleo_utils::block_processor::IdValuePair;
use aleo_utils::json_like;
use aleo_utils::string_encoder::StringEncoder;
use async_trait::async_trait;
use error_stack::{ensure, report, Report, Result, ResultExt};
use mockall::automock;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use thiserror::Error;
use tracing::info;

use crate::types::Hash;
use crate::url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create client")]
    Client,
    #[error("Request error")]
    Request,
    #[error("Transaction '{0}' not found")]
    TransactionNotFound(String),
    #[error("Transition '{0}' not found")]
    TransitionNotFound(String),
    #[error("Failed to find callContract")]
    CallnotFound,
    #[error("Failed to find user call")]
    UserCallnotFound,
    #[error("The provided chain name is invalid")]
    InvalidChainName,
    #[error("Invalid source address")]
    InvalidSourceAddress,
    #[error("Failed to create AleoID: {0}")]
    FailedToCreateAleoID(String),
}

#[derive(Debug)]
pub enum Receipt {
    Found(TransitionReceipt),
    NotFound(Transition, Report<Error>),
}

#[derive(Debug)]
pub struct TransitionReceipt {
    pub transition: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address,
    pub payload: Vec<u8>,
}

impl PartialEq<crate::handlers::aleo_verify_msg::Message> for TransitionReceipt {
    fn eq(&self, message: &crate::handlers::aleo_verify_msg::Message) -> bool {
        info!(
            "transition_id: chain.{} == msg.{} ({})",
            self.transition,
            message.tx_id,
            self.transition == message.tx_id
        );
        info!(
            "destination_address: chain.{} == msg.{} ({})",
            self.destination_address,
            message.destination_address,
            self.destination_address == message.destination_address
        );
        info!(
            "destination_chain: chain.{} == msg.{} ({})",
            self.destination_chain,
            message.destination_chain,
            self.destination_chain == message.destination_chain
        );
        info!(
            "source_address: chain.{:?} == msg.{:?} ({})",
            self.source_address,
            message.source_address,
            self.source_address == message.source_address
        );

        let payload = std::str::from_utf8(&self.payload).unwrap();
        let payload_hash = if self.destination_chain.as_ref().starts_with("eth") {
            let payload = json_like::into_json(payload).unwrap();
            let payload: Vec<u8> = serde_json::from_str(&payload).unwrap();
            let payload = std::str::from_utf8(&payload).unwrap();
            let payload = solabi::encode(&payload);
            let payload_hash = keccak256(&payload).to_vec();
            Hash::from_slice(&payload_hash)
        } else {
            // Keccak + bhp hash
            let payload_hash =
                aleo_gateway::aleo_hash::<&str, snarkvm_cosmwasm::network::TestnetV0>(payload)
                    .unwrap();
            let payload_hash = payload_hash.strip_suffix("group").unwrap();
            let hash = cosmwasm_std::Uint256::from_str(&payload_hash).unwrap();
            let hash = hash.to_le_bytes();
            Hash::from_slice(&hash)
        };

        info!(
            "payload_hash: chain.{:?} == msg.{:?} ({})",
            payload_hash,
            message.payload_hash,
            payload_hash == message.payload_hash
        );

        self.transition == message.tx_id
            && self.destination_address == message.destination_address
            && self.destination_chain == message.destination_chain
            && self.source_address == message.source_address
            && payload_hash == message.payload_hash
    }
}

#[automock]
#[async_trait]
pub trait ClientTrait: Send {
    async fn get_transaction(
        &self,
        transaction_id: &Transaction,
    ) -> Result<aleo_utils::block_processor::Transaction, Error>;

    async fn find_transaction(&self, transition_id: &Transition) -> Result<String, Error>;
}

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: Url,
    network: String,
}

#[derive(Default, Debug)]
struct ParsedOutput {
    payload: Vec<u8>,
    call_contract: CallContract,
}

impl Client {
    pub fn new(client: reqwest::Client, base_url: Url, network: String) -> Self {
        Self {
            client,
            base_url,
            network,
        }
    }
}

#[async_trait]
impl ClientTrait for Client {
    #[tracing::instrument(skip(self))]
    async fn get_transaction(
        &self,
        transaction_id: &Transaction,
    ) -> Result<aleo_utils::block_processor::Transaction, Error> {
        const ENDPOINT: &str = "transaction";
        let url = format!(
            "{}{}/{ENDPOINT}/{}",
            self.base_url, self.network, &transaction_id
        );

        tracing::debug!(%url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .change_context(Error::Request)?;

        let transaction: aleo_utils::block_processor::Transaction =
            serde_json::from_str(&response.text().await.change_context(Error::Request)?)
                .change_context(Error::Request)?;

        Ok(transaction)
    }

    #[tracing::instrument(skip(self))]
    async fn find_transaction(&self, transition_id: &Transition) -> Result<String, Error> {
        const ENDPOINT: &str = "find/transactionID";
        let url = format!(
            "{}{}/{ENDPOINT}/{}",
            self.base_url, self.network, &transition_id
        );
        tracing::debug!(%url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .change_context(Error::Request)?;

        response.text().await.change_context(Error::Request)
    }
}

pub struct ClientWrapper<'a, C: ClientTrait> {
    client: &'a C,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CallContract {
    pub(crate) caller: String,
    pub(crate) sender: String,
    pub(crate) destination_chain: Vec<u128>,
    pub(crate) destination_address: Vec<u128>,
}

impl CallContract {
    pub fn destination_chain(&self) -> String {
        let encoded_string = StringEncoder {
            buf: self.destination_chain.clone(),
        };
        encoded_string.decode()
    }

    pub fn destination_address(&self) -> Result<String, error_stack::Report<Error>> {
        let encoded_string = StringEncoder {
            buf: self.destination_address.clone(),
        };
        let ascii_string = encoded_string.decode();
        Ok(ascii_string)
    }
}

impl<'a, C> ClientWrapper<'a, C>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn new(client: &'a C) -> Self {
        Self { client }
    }

    fn find_call_contract(&self, outputs: &[IdValuePair]) -> Option<CallContract> {
        if outputs.len() != 1 {
            return None;
        }

        outputs
            .first()
            .and_then(|o| match o {
                IdValuePair {
                    id: _,
                    value: Some(value),
                } => Some(value),
                _ => None,
            })
            .and_then(|value| {
                let json = json_like::into_json(value.to_string().as_str()).unwrap();
                serde_json::from_str::<CallContract>(&json).ok()
            })
    }

    fn parse_user_output(&self, outputs: &[IdValuePair]) -> Result<ParsedOutput, Error> {
        if outputs.len() != 2 {
            return Err(Report::new(Error::UserCallnotFound));
        }

        // ParsedOutput
        let mut parsed_output = ParsedOutput::default();

        // TODO: there mast be a better way
        for o in outputs {
            if let IdValuePair {
                id: _,
                value: Some(plaintext),
            } = o
            {
                let json = json_like::into_json(plaintext.to_string().as_str()).unwrap();
                let parsed = serde_json::from_str::<CallContract>(&json);

                if let Ok(call_contract) = parsed {
                    parsed_output.call_contract = call_contract;
                } else {
                    parsed_output.payload = plaintext.to_string().as_bytes().to_vec();
                }
            }
        }

        Ok(parsed_output)
    }

    #[tracing::instrument(skip(self))]
    pub async fn transition_receipt(
        &self,
        transition_id: &Transition,
        gateway_contract: &str,
    ) -> Result<Receipt, Error> {
        let transaction = self.client.find_transaction(transition_id).await?;
        let transaction = transaction.trim_matches('"');

        // Find transaction
        let transaction_id = Transaction::from_str(transaction)
            .change_context(Error::TransactionNotFound(transaction.to_string()))?;

        let transaction = self.client.get_transaction(&transaction_id).await?;
        let gateway_transition = transaction
            .execution
            .transitions
            .iter()
            .find(|t| t.program.as_str() == gateway_contract)
            .ok_or(Error::TransitionNotFound(transition_id.to_string()))?;

        // Get the outputs of the transition
        // The transition should have only the gateway call
        let outputs = &gateway_transition.outputs;
        let call_contract_call = self.find_call_contract(&outputs);
        let call_contract = call_contract_call.ok_or(Error::CallnotFound)?;

        let scm = gateway_transition.scm.as_str();

        let gateway_calls_count = transaction
            .execution
            .transitions
            .iter()
            .filter(|t| t.scm == scm && t.program == gateway_contract)
            .count();

        ensure!(gateway_calls_count == 1, Error::CallnotFound);

        let same_scm: Vec<_> = transaction
            .execution
            .transitions
            .iter()
            .filter(|t| t.scm == scm && t.id != gateway_transition.id)
            .collect();

        ensure!(gateway_calls_count == 1, Error::UserCallnotFound);

        let parsed_output = self.parse_user_output(&same_scm[0].outputs)?;

        ensure!(
            parsed_output.call_contract == call_contract,
            Error::UserCallnotFound
        );

        Ok(Receipt::Found(TransitionReceipt {
            transition: transition_id.clone(),
            destination_address: call_contract
                .destination_address()
                .map_err(|e| Report::new(Error::FailedToCreateAleoID(e.to_string())))?,
            destination_chain: ChainName::try_from(call_contract.destination_chain())
                .change_context(Error::InvalidChainName)?,
            source_address: Address::from_str(call_contract.sender.to_string().as_ref())
                .change_context(Error::InvalidSourceAddress)?,
            payload: parsed_output.payload,
        }))
    }
}

fn keccak256(payload: impl AsRef<[u8]>) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(payload);
    hasher.finalize().into()
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use std::ops::Deref;
    use std::str::FromStr;

    use super::*;

    pub fn mock_client() -> MockClientTrait {
        let mut mock_client = MockClientTrait::new();

        let transaction_id = "at18c83pwjlvvjpdk95pudngzxqydvq92np206njcyppgndjalujsrshjn48j";
        let mut expected_transitions: HashMap<
            Transaction,
            aleo_utils::block_processor::Transaction,
        > = HashMap::new();
        let transaction_one = include_str!(
            "../tests/at18c83pwjlvvjpdk95pudngzxqydvq92np206njcyppgndjalujsrshjn48j.json"
        );
        let snark_tansaction: aleo_utils::block_processor::Transaction =
            serde_json::from_str(transaction_one).unwrap();
        let transaction = Transaction::from_str(transaction_id).unwrap();
        expected_transitions.insert(transaction, snark_tansaction);

        mock_client
            .expect_get_transaction()
            .returning(move |transaction| {
                Ok(expected_transitions.get(transaction).unwrap().clone())
            });

        mock_client.expect_find_transaction().returning(move |_| {
            let transaction_id = "at18c83pwjlvvjpdk95pudngzxqydvq92np206njcyppgndjalujsrshjn48j";
            Ok(transaction_id.to_string())
        });

        mock_client
    }

    pub fn mock_client_2() -> MockClientTrait {
        let mut mock_client = MockClientTrait::new();

        let transaction_id = "at1dgmvx30f79wt6w8fcjurwtsc5zak4efg4ayyme79862xylve7gxsq3nfh6";
        let mut expected_transitions: HashMap<
            Transaction,
            aleo_utils::block_processor::Transaction,
        > = HashMap::new();
        let transaction_one = include_str!(
            "../tests/at1dgmvx30f79wt6w8fcjurwtsc5zak4efg4ayyme79862xylve7gxsq3nfh6.json"
        );
        let snark_tansaction: aleo_utils::block_processor::Transaction =
            serde_json::from_str(transaction_one).unwrap();
        let transaction = Transaction::from_str(transaction_id).unwrap();
        expected_transitions.insert(transaction, snark_tansaction);

        mock_client
            .expect_get_transaction()
            .returning(move |transaction| {
                Ok(expected_transitions.get(transaction).unwrap().clone())
            });

        mock_client.expect_find_transaction().returning(move |_| {
            let transaction_id = "at1dgmvx30f79wt6w8fcjurwtsc5zak4efg4ayyme79862xylve7gxsq3nfh6";
            Ok(transaction_id.to_string())
        });

        mock_client
    }

    pub fn mock_client_3() -> MockClientTrait {
        let mut mock_client = MockClientTrait::new();

        let transaction_id = "at14gry4nauteg5sp00p6d2pj93dhpsm5857ml8y3xg57nkpszhav9qk0tgvd";
        let mut expected_transitions: HashMap<
            Transaction,
            aleo_utils::block_processor::Transaction,
        > = HashMap::new();
        let transaction_one = include_str!(
            "../tests/at14gry4nauteg5sp00p6d2pj93dhpsm5857ml8y3xg57nkpszhav9qk0tgvd.json"
        );
        let snark_tansaction: aleo_utils::block_processor::Transaction =
            serde_json::from_str(transaction_one).unwrap();
        let transaction = Transaction::from_str(transaction_id).unwrap();
        expected_transitions.insert(transaction, snark_tansaction);

        mock_client
            .expect_get_transaction()
            .returning(move |transaction| {
                Ok(expected_transitions.get(transaction).unwrap().clone())
            });

        mock_client.expect_find_transaction().returning(move |_| {
            let transaction_id = "at14gry4nauteg5sp00p6d2pj93dhpsm5857ml8y3xg57nkpszhav9qk0tgvd";
            Ok(transaction_id.to_string())
        });

        mock_client
    }

    #[tokio::test]
    async fn foo_test() {
        let client = mock_client_2();
        let transision_id = "au1zn24gzpgkr936qv49g466vfccg8aykcv05rk39s239hjxwrtsu8sltpsd8";
        let transition = Transition::from_str(transision_id).unwrap();
        let client = ClientWrapper::new(&client);
        let gateway_contract = "gateway_base.aleo";

        let res = client
            .transition_receipt(&transition, gateway_contract)
            .await;
        println!("{res:#?}");
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn flow_test() {
        let client = mock_client_3();
        let transision_id = "au17kdp7a7p6xuq6h0z3qrdydn4f6fjaufvzvlgkdd6vzpr87lgcgrq8qx6st";
        let transition = Transition::from_str(transision_id).unwrap();
        let client = ClientWrapper::new(&client);
        let gateway_contract = "ac64caccf8221554ec3f89bf.aleo";

        let res = client
            .transition_receipt(&transition, gateway_contract)
            .await;
        assert!(res.is_ok());
    }
}
