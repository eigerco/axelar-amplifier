use std::str::FromStr;

use aleo_gateway::WeightedSigners;
use aleo_types::address::Address;
use aleo_types::program::Program;
use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use aleo_utils::block_processor::IdValuePair;
use aleo_utils::json_like;
use aleo_utils::string_encoder::StringEncoder;
use async_trait::async_trait;
use error_stack::{ensure, Report, Result, ResultExt};
use mockall::automock;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use thiserror::Error;
use tracing::{error, info};

use crate::types::Hash;
use crate::url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Request error")]
    Request,
    #[error("Transaction '{0}' not found")]
    TransactionNotFound(String),
    #[error("Transition '{0}' not found")]
    TransitionNotFound(String),
    #[error("Failed to find callContract")]
    CallContractNotFound,
    #[error("Failed to find user call")]
    UserCallnotFound,
    #[error("The provided chain name is invalid")]
    InvalidChainName,
    #[error("Invalid source address")]
    InvalidSourceAddress,
    #[error("Failed to create AleoID: {0}")]
    FailedToCreateAleoID(String),
    #[error("Failed to create hash payload: {0}")]
    PayloadHash(String),
    #[error("Failed to find transition '{0}' in transaction '{1}'")]
    TransitionNotFoundInTransaction(String, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Initial;

#[derive(Debug, Serialize, Deserialize)]
pub struct StateTanstitionId {
    transition_id: Transition,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateTransactionId {
    transaction_id: Transaction,
}

#[derive(Debug, Deserialize)]
pub struct StateTransactionFound {
    transaction: aleo_utils::block_processor::Transaction,
}

#[derive(Debug, Deserialize)]
pub struct StateTransitionFound {
    transaction: aleo_utils::block_processor::Transaction,
    transition: aleo_utils::block_processor::Transition,
}

pub struct Driver<'a, C: ClientTrait, S> {
    client: &'a C,
    target_contract: Program, // The target program can be the CallContract or the
    state: S,
}

impl<'a, C> Driver<'a, C, Initial>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn new(client: &'a C, target_contract: Program) -> Self {
        Self {
            client,
            target_contract,
            state: Initial,
        }
    }

    pub async fn get_transaction_id(
        self,
        transition_id: Transition,
    ) -> Result<Driver<'a, C, StateTransactionId>, Error> {
        let transaction_id = self
            .client
            .find_transaction(&transition_id)
            .await
            .change_context(Error::TransitionNotFound(transition_id.to_string()))?;

        let transaction = transaction_id.trim_matches('"');

        Ok(Driver {
            client: self.client,
            target_contract: self.target_contract.clone(),
            state: StateTransactionId {
                transaction_id: Transaction::from_str(transaction)
                    .change_context(Error::TransactionNotFound(transaction.to_string()))?,
            },
        })
    }
}

impl<'a, C> Driver<'a, C, StateTransactionId>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub async fn get_transaction(self) -> Result<Driver<'a, C, StateTransactionFound>, Error> {
        let transaction = self
            .client
            .get_transaction(&self.state.transaction_id)
            .await
            .change_context(Error::TransactionNotFound(
                self.state.transaction_id.to_string(),
            ))?;

        Ok(Driver {
            client: self.client,
            target_contract: self.target_contract.clone(),
            state: StateTransactionFound { transaction },
        })
    }
}

impl<'a, C> Driver<'a, C, StateTransactionFound>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn get_transition(self) -> Result<Driver<'a, C, StateTransitionFound>, Error> {
        let transition = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .find(|t| t.program.as_str() == self.target_contract.as_str())
            .ok_or(Error::TransitionNotFoundInTransaction("foo".to_string(), "bar".to_string()))?
            .clone(); // TODO: remove clone

        Ok(Driver {
            client: self.client,
            target_contract: self.target_contract.clone(),
            state: StateTransitionFound {
                transaction: self.state.transaction,
                transition,
            },
        })
    }
}

impl<'a, C> Driver<'a, C, StateTransitionFound>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn check_call_contract(self) -> Result<Receipt, Error> {
        let outputs = self.state.transition.outputs;
        let call_contract = find_call_contract(&outputs).ok_or(Error::CallContractNotFound)?;
        let scm = self.state.transition.scm.as_str();

        let gateway_calls_count = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .filter(|t| t.scm == scm && t.program == self.target_contract.as_str())
            .count();

        ensure!(gateway_calls_count == 1, Error::CallContractNotFound);

        let same_scm: Vec<_> = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .filter(|t| t.scm == scm && t.id != self.state.transition.id)
            .collect();

        ensure!(same_scm.len() == 1, Error::UserCallnotFound);

        let parsed_output =
            parse_user_output(&same_scm[0].outputs).change_context(Error::UserCallnotFound)?;

        ensure!(
            parsed_output.call_contract == call_contract,
            Error::UserCallnotFound
        );

        Ok(Receipt::Found(FoundReceipt::CallContract(
            CallContractReceipt {
                transition: Transition::from_str(self.state.transition.id.as_str()).unwrap(),
                destination_address: call_contract
                    .destination_address()
                    .map_err(|e| Report::new(Error::FailedToCreateAleoID(e.to_string())))?,
                destination_chain: ChainName::try_from(call_contract.destination_chain())
                    .change_context(Error::InvalidChainName)?,
                source_address: Address::from_str(call_contract.sender.to_string().as_ref())
                    .change_context(Error::InvalidSourceAddress)?,
                payload: parsed_output.payload,
            },
        )))
    }
    pub fn check_signer_rotation(self) -> Result<Receipt, Error> {
        let outputs = self.state.transition.outputs;
        let signer_rotation = find_signer_rotation(&outputs).ok_or(Error::CallContractNotFound)?;
        let scm = self.state.transition.scm.as_str();

        // let gateway_calls_count = self
        //     .state
        //     .transaction
        //     .execution
        //     .transitions
        //     .iter()
        //     .filter(|t| t.scm == scm && t.program == self.target_contract.as_str())
        //     .count();
        //
        // ensure!(gateway_calls_count == 1, Error::CallContractNotFound);
        //
        // let same_scm: Vec<_> = self
        //     .state
        //     .transaction
        //     .execution
        //     .transitions
        //     .iter()
        //     .filter(|t| t.scm == scm && t.id != self.state.transition.id)
        //     .collect();
        //
        // ensure!(same_scm.len() == 1, Error::UserCallnotFound);
        //
        // let parsed_output =
        //     parse_user_output(&same_scm[0].outputs).change_context(Error::UserCallnotFound)?;
        //
        // ensure!(
        //     parsed_output.call_contract == call_contract,
        //     Error::UserCallnotFound
        // );
        //
        Ok(Receipt::Found(FoundReceipt::SignerRotation(
            SignerRotationReceipt {},
        )))
    }
}

fn parse_user_output(outputs: &[IdValuePair]) -> Result<ParsedOutput, Error> {
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

fn find_call_contract(outputs: &[IdValuePair]) -> Option<CallContract> {
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

fn find_signer_rotation(outputs: &[IdValuePair]) -> Option<SignerRotation> {
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
            serde_json::from_str::<SignerRotation>(&json).ok()
        })
}

#[derive(Debug)]
pub enum Receipt {
    Found(FoundReceipt),
    NotFound(Transition, Report<Error>),
}

#[derive(Debug)]
pub enum FoundReceipt {
    CallContract(CallContractReceipt),
    SignerRotation(SignerRotationReceipt),
}

#[derive(Debug)]
pub struct CallContractReceipt {
    pub transition: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address,
    pub payload: Vec<u8>,
}

impl PartialEq<crate::handlers::aleo_verify_msg::Message> for CallContractReceipt {
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

        let payload_hash = match payload_hash(&self.payload, self.destination_chain.as_ref()) {
            Ok(hash) => hash,
            Err(e) => {
                error!("payload_hash: {}", e);
                return false;
            }
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

#[derive(Debug, PartialEq, Eq)]
pub struct SignerRotationReceipt {

}

// impl PartialEq<crate::handlers::aleo_verify_msg::Message> for SignerRotationReceipt {
//     fn eq(&self, message: &crate::handlers::aleo_verify_verifier_set::VerifierSetConfirmation) -> bool {
//         true
//     }
// }

fn payload_hash(payload: &[u8], destination_chain: &str) -> std::result::Result<Hash, Error> {
    let payload = std::str::from_utf8(payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
    let payload_hash = if destination_chain.starts_with("eth") {
        let payload =
            json_like::into_json(payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload: Vec<u8> =
            serde_json::from_str(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload =
            std::str::from_utf8(&payload).map_err(|e| Error::PayloadHash(e.to_string()))?;
        let payload = solabi::encode(&payload);
        let payload_hash = keccak256(&payload).to_vec();
        Hash::from_slice(&payload_hash)
    } else {
        // Keccak + bhp hash
        let payload_hash =
            aleo_gateway::hash::<&str, snarkvm_cosmwasm::network::TestnetV0>(payload)
                .map_err(|e| Error::PayloadHash(e.to_string()))?;
        Hash::from_slice(&payload_hash)
    };

    Ok(payload_hash)
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

#[derive(Debug, Deserialize)]
pub struct SignerRotation {
    pub(crate) block_height: u32,
    pub(crate) signers_hash: String,
    pub(crate) weighted_signers: WeightedSigners,
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

fn keccak256(payload: impl AsRef<[u8]>) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(payload);
    hasher.finalize().into()
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use super::*;

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

    // #[tokio::test]
    // async fn foo_test() {
    //     let client = mock_client_2();
    //     let transision_id = "au1zn24gzpgkr936qv49g466vfccg8aykcv05rk39s239hjxwrtsu8sltpsd8";
    //     let transition = Transition::from_str(transision_id).unwrap();
    //     let client = ClientWrapper::new(&client);
    //     let gateway_contract = "gateway_base.aleo";
    //
    //     let res = client
    //         .transition_receipt(&transition, gateway_contract)
    //         .await;
    //     assert!(res.is_ok());
    // }
    //
    // #[tokio::test]
    // async fn flow_test1() {
    //     let client = mock_client_3();
    //     let transision_id = "au17kdp7a7p6xuq6h0z3qrdydn4f6fjaufvzvlgkdd6vzpr87lgcgrq8qx6st";
    //     let transition = Transition::from_str(transision_id).unwrap();
    //     let client = ClientWrapper::new(&client);
    //     let gateway_contract = "ac64caccf8221554ec3f89bf.aleo";
    //
    //     let res = client
    //         .transition_receipt(&transition, gateway_contract)
    //         .await;
    //     println!("res: {:#?}", res);
    //     assert!(res.is_ok());
    // }

    #[tokio::test]
    async fn flow_test2() {
        let client = mock_client_3();
        let transision_id = "au17kdp7a7p6xuq6h0z3qrdydn4f6fjaufvzvlgkdd6vzpr87lgcgrq8qx6st";
        let transition = Transition::from_str(transision_id).unwrap();
        let gateway_contract = "ac64caccf8221554ec3f89bf.aleo";

        let driver = Driver::new(&client, Program::from_str(gateway_contract).unwrap());

        let driver = driver.get_transaction_id(transition.clone()).await.unwrap();
        let driver = driver.get_transaction().await.unwrap();
        let driver = driver.get_transition().unwrap();
        let receipt = driver.check_call_contract().unwrap();

        println!("driver: {:#?}", receipt);
    }
}
