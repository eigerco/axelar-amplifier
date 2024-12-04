use std::str::FromStr;

use aleo_types::address::Address;
use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use async_trait::async_trait;
use error_stack::{ensure, report, Report, Result, ResultExt};
use mockall::automock;
use router_api::ChainName;
use snarkvm::ledger::{Output, Transaction as SnarkvmTransaction};
use snarkvm::prelude::{AleoID, Field, TestnetV0};
use thiserror::Error;
use tracing::warn;

use super::parser::CallContract;
use crate::types::Hash;

type CurrentNetwork = TestnetV0;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create client")]
    Client,
    #[error("Request error")]
    Request,
    #[error("Transition not found")]
    TransitionNotFound,
    #[error("Not an execution transition")]
    TransactionNotexecution,
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

type Payload = String;

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
    pub payload: Payload,
}

impl PartialEq<crate::handlers::aleo_verify_msg::Message> for TransitionReceipt {
    fn eq(&self, message: &crate::handlers::aleo_verify_msg::Message) -> bool {
        use sha3::Digest;
        let mut hasher = sha3::Keccak256::new();
        hasher.update(self.payload.clone());
        let result = hasher.finalize();
        let payload_hash = Hash::from_slice(result.as_slice());

        self.transition == message.transition_id
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
    ) -> Result<SnarkvmTransaction<CurrentNetwork>, Error>;

    async fn find_transaction(&self, transition_id: &Transition) -> Result<String, Error>; // TODO: remove magic number
}

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: String,
    network: String,
}

#[derive(Default, Debug)]
struct ParsedOutput {
    payload: String,
    call_contract: CallContract,
}

impl Client {
    pub fn new(client: reqwest::Client, base_url: String, network: String) -> Result<Self, Error> {
        ensure!(
            base_url.starts_with("http://") || base_url.starts_with("https://"),
            report!(Error::Client).attach_printable("specified url {base_url} invalid, the base url must start with or https:// (or http:// if doing local development)")
        );

        Ok(Self {
            client,
            base_url,
            network,
        })
    }
}

#[async_trait]
impl ClientTrait for Client {
    async fn get_transaction(
        &self,
        transaction_id: &Transaction,
    ) -> Result<SnarkvmTransaction<CurrentNetwork>, Error> {
        const ENDPOINT: &str = "transaction";
        let url = format!(
            "{}/{}/{ENDPOINT}/{}",
            self.base_url, self.network, &transaction_id
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .change_context(Error::Request)?;

        let transaction: SnarkvmTransaction<CurrentNetwork> =
            serde_json::from_str(&response.text().await.change_context(Error::Request)?)
                .change_context(Error::Request)?; // TODO: This is a CPU intensive operation. We need to handle it differently

        Ok(transaction)
    }

    async fn find_transaction(&self, transition_id: &Transition) -> Result<String, Error> {
        const ENDPOINT: &str = "find/transactionID";
        let url = format!(
            "{}/{}/{ENDPOINT}/{}",
            self.base_url, self.network, &transition_id
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .change_context(Error::Request)?;

        response.text().await.change_context(Error::Request)
    }
}

pub struct ClientWrapper<C: ClientTrait> {
    client: C,
}

impl<C> ClientWrapper<C>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn new(client: C) -> Self {
        Self { client }
    }

    fn find_call_contract(&self, outputs: &[Output<CurrentNetwork>]) -> Option<CallContract> {
        if outputs.len() != 1 {
            return None;
        }

        outputs
            .first()
            .and_then(|o| match o {
                Output::<CurrentNetwork>::Public(_field, Some(plaintext)) => {
                    Some(plaintext.to_string())
                }
                _ => None,
            })
            .as_ref()
            .and_then(|value| crate::aleo::parser::parse_call_contract(value))
    }

    fn parse_user_output(&self, outputs: &[Output<CurrentNetwork>]) -> Result<ParsedOutput, Error> {
        if outputs.len() != 2 {
            return Err(Report::new(Error::UserCallnotFound));
        }

        // ParsedOutput
        let mut parsed_output = ParsedOutput::default();

        // TODO: there mast be a better way
        for o in outputs {
            if let Output::<CurrentNetwork>::Public(_field, Some(plaintext)) = o {
                let parsed =
                    crate::aleo::parser::parse_call_contract(plaintext.to_string().as_str());
                if let Some(call_contract) = parsed {
                    parsed_output.call_contract = call_contract;
                } else {
                    parsed_output.payload = plaintext.to_string();
                }
            }
        }

        Ok(parsed_output)
    }

    pub async fn transition_receipt(
        &self,
        transition_id: &Transition,
        gateway_contract: &str,
    ) -> Result<Receipt, Error> {
        const TRANSITION_PREFIX: &[u8] = "au".as_bytes();
        const TRANSITION_BYTES_PREFIX: u16 =
            u16::from_le_bytes([TRANSITION_PREFIX[0], TRANSITION_PREFIX[1]]);

        // Find transaction
        let transaction_id =
            Transaction::from_str(self.client.find_transaction(transition_id).await?.as_str())
                .change_context(Error::TransitionNotFound)?;

        let transaction = self.client.get_transaction(&transaction_id).await?;

        if transaction.execution().is_none() {
            warn!("Transaction '{:?}' is not an execution transaction. The following transitions can not be vailidated: '{:?}'",
                transaction_id,
                transition_id
            );
            return Err(Report::new(Error::TransactionNotexecution));
        }

        // Get the gateway transition
        let gateway_transition = transaction
            .find_transition(
                &AleoID::<Field<CurrentNetwork>, TRANSITION_BYTES_PREFIX>::from_str(
                    transition_id.to_string().as_str(),
                )
                .map_err(|e| Error::FailedToCreateAleoID(e.to_string()))?,
            )
            .ok_or(Error::TransitionNotFound)?;

        // Get the outputs of the transition
        // The transition should have only the gateway call
        let outputs = gateway_transition.outputs();
        let call_contract_call = self.find_call_contract(outputs);
        let call_contract = call_contract_call.ok_or(Error::CallnotFound)?;

        let scm = gateway_transition.scm();

        let gateway_calls_count = transaction
            .transitions()
            .filter(|t| t.scm() == scm && t.program_id().to_string().as_str() == gateway_contract)
            .count();

        ensure!(gateway_calls_count == 1, Error::CallnotFound);

        let same_scm: Vec<_> = transaction
            .transitions()
            .filter(|t| t.scm() == scm && t.id() != gateway_transition.id())
            .collect();

        ensure!(gateway_calls_count == 1, Error::UserCallnotFound);

        let parsed_output = self.parse_user_output(same_scm[0].outputs())?;

        ensure!(
            parsed_output.call_contract == call_contract,
            Error::UserCallnotFound
        );

        Ok(Receipt::Found(TransitionReceipt {
            transition: transition_id.clone(),
            destination_address: format!("{:02X?}", &call_contract.destination_address),
            destination_chain: ChainName::try_from(call_contract.destination_chain())
                .change_context(Error::InvalidChainName)?,
            source_address: Address::from_str(call_contract.sender.to_string().as_ref())
                .change_context(Error::InvalidSourceAddress)?,
            payload: parsed_output.payload,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use super::*;

    fn mock_client() -> MockClientTrait {
        let mut mock_client = MockClientTrait::new();

        let transaction_id = "at18c83pwjlvvjpdk95pudngzxqydvq92np206njcyppgndjalujsrshjn48j";
        let mut expected_transitions: HashMap<Transaction, SnarkvmTransaction<CurrentNetwork>> =
            HashMap::new();
        let transaction_one = include_str!(
            "../tests/at18c83pwjlvvjpdk95pudngzxqydvq92np206njcyppgndjalujsrshjn48j.json"
        );
        let snark_tansaction =
            SnarkvmTransaction::<CurrentNetwork>::from_str(transaction_one).unwrap();
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

    #[tokio::test]
    async fn foo_test() {
        let client = mock_client();
        let transision_id = "au1knlxwe55dx6cnm2j5sgtsl2z2z590jprme2t4cc49h85uv0emgrsuzvutv";
        let transition = Transition::from_str(transision_id).unwrap();
        let client = ClientWrapper::new(client);
        let gateway_contract = "vzevxifdoj.aleo";

        let res = client
            .transition_receipt(&transition, gateway_contract)
            .await;
        println!("{res:#?}");
    }
}