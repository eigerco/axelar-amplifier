use std::collections::HashMap;
use std::str::FromStr;

use aleo_types::address::Address;
use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use async_trait::async_trait;
use cosmwasm_std::ensure;
use error_stack::{report, Report, Result, ResultExt};
use futures::stream::{self, StreamExt};
use mockall::automock;
use router_api::ChainName;
use serde::Deserialize;
use snarkvm::ledger::{Output, Transaction as SnarkvmTransaction};
use snarkvm::prelude::{Address as AleoAddress, AleoID, Field, TestnetV0};
use thiserror::Error;
use tracing::warn;

use crate::types::Hash;

type CurrentNetwork = TestnetV0;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create client")]
    Client,
    #[error("Request error")]
    Request,
    #[error("Transition not found in transaction {0}")]
    TransitionNotFound(String),
    #[error("Failed to parse call contract")]
    FailedParseCallContract,
    #[error("Currently only one gateway call is supported")]
    MoreThanOneGatewayCall,
    #[error("The provided chain name is invalid")]
    InvalidChainName,
    #[error("Invalid source address")]
    InvalidSourceAddress,
    #[error("Failed to create Aleo transaction id")]
    InvalidAleoTransactionId,
    #[error("Failed to create AleoID: {0}")]
    FailedToCreateAleoID(String),
    #[error("HttpError")]
    HttpError,
}

type Payload = String;

#[derive(Debug)]
pub enum Receipt {
    Found(TransitionReceipt),
    NotFound(Transaction, Transition, Report<Error>),
}

#[derive(Debug)]
pub struct TransitionReceipt {
    pub transaction: Transaction,
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

        self.transaction == message.transaction_id
            && self.transition == message.transition_id
            && self.destination_address == message.destination_address
            && self.destination_chain == message.destination_chain
            // && self.source_address == message.source_address // TODO
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
}

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: String,
    network: String,
}

enum TempParsedOutput {
    Payload(String),
    CallContract(CallContract),
}

struct ParsedOutput {
    payload: String,
    call_contract: CallContract,
}

#[derive(Debug, Deserialize)]
struct CallContract {
    #[allow(dead_code)]
    caller: AleoAddress<CurrentNetwork>,
    sender: AleoAddress<CurrentNetwork>,
    destination_address: Vec<u8>,
    destination_chain: Vec<u8>,
}

impl CallContract {
    fn destination_chain(&self) -> String {
        self.destination_chain
            .iter()
            .take_while(|&&value| value != 0) // Stop at the first zero
            .map(|&value| value as char) // Convert to characters
            .collect()
    }
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

    fn parse_outputs(&self, outputs: &[Output<CurrentNetwork>]) -> Result<ParsedOutput, Error> {
        // need to find one payload and one call contract
        // Check if the output is call contract
        //  if its not a call contract assume its payload
        //  if one of them is call contract assume is correct

        let mut outputs = outputs
            .iter()
            .flat_map(|o| match o {
                Output::<CurrentNetwork>::Public(_field, Some(plaintext)) => {
                    Some(plaintext.to_string())
                }
                _ => None,
            })
            .map(|value| {
                if value.contains("caller")
                    && value.contains("sender")
                    && value.contains("destination_chain")
                    && value.contains("destination_address")
                {
                    let value = value
                        .replace("\n", "")
                        .replace("caller", r#""caller""#)
                        .replace("sender", r#""sender""#)
                        .replace("destination_chain", r#""destination_chain""#)
                        .replace("destination_address", r#""destination_address""#)
                        .replace(
                            "aleo1gtsw3kdzm86mmmzf0spk5j840g8ln0y902fa3duwh0we0jv2g5xqza9ag9",
                            r#""aleo1gtsw3kdzm86mmmzf0spk5j840g8ln0y902fa3duwh0we0jv2g5xqza9ag9""#,
                        )
                        .replace(
                            "aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau",
                            r#""aleo10fmsqwh059uqm74x6t6zgj93wfxtep0avevcxz0n4w9uawymkv9s7whsau""#,
                        )
                        .replace("u8", "");

                    println!("veclue -->{value:?}<--");
                    Ok(TempParsedOutput::CallContract(
                        serde_json::from_str(&value)
                            .change_context(Error::FailedParseCallContract)?,
                    ))
                } else {
                    Ok(TempParsedOutput::Payload(value))
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // ensure that there are only two outputs and one of them is CallContract and the other one is Payload
        ensure!(outputs.len() == 2, Error::FailedParseCallContract);
        ensure!(
            outputs
                .iter()
                .any(|o| matches!(o, TempParsedOutput::CallContract(_))),
            Error::FailedParseCallContract
        );
        ensure!(
            outputs
                .iter()
                .any(|o| matches!(o, TempParsedOutput::Payload(_))),
            Error::FailedParseCallContract
        );

        if matches!(outputs[0], TempParsedOutput::CallContract(_)) {
            let TempParsedOutput::CallContract(call_contract) = outputs.remove(0) else {
                todo!();
            };
            let TempParsedOutput::Payload(payload) = outputs.remove(0) else {
                todo!();
            };

            Ok(ParsedOutput {
                call_contract,
                payload,
            })
        } else {
            let TempParsedOutput::CallContract(call_contract) = outputs.remove(1) else {
                todo!();
            };
            let TempParsedOutput::Payload(payload) = outputs.remove(0) else {
                todo!();
            };

            Ok(ParsedOutput {
                call_contract,
                payload,
            })
        }
    }

    async fn transition_receipt(
        &self,
        transaction: &SnarkvmTransaction<CurrentNetwork>,
        transition_id: &Transition,
    ) -> Result<TransitionReceipt, Error> {
        const P: &[u8] = "au".as_bytes();
        const PREFIX: u16 = u16::from_le_bytes([P[0], P[1]]);

        let transition = transaction
            .find_transition(
                &AleoID::<Field<CurrentNetwork>, PREFIX>::from_str(transition_id.to_string().as_str())
                    .map_err(|e| Error::FailedToCreateAleoID(e.to_string()))?,
            )
            .ok_or(Error::TransitionNotFound(transaction.id().to_string()))?;

        // Get the outputs of the transition
        // The transition should have exactly one payload and one call to the gateway contract
        let outputs = transition.outputs();
        let parsed_outputs = self.parse_outputs(outputs)?;
        let scm = transition.scm();

        let gateway_calls_count = transaction
            .transitions()
            .filter(|t| t.scm() == scm && t.program_id().to_string().as_str() == "gateway.aleo")
            .count();

        ensure!(gateway_calls_count == 1, Error::MoreThanOneGatewayCall);

        Ok(TransitionReceipt {
            transaction: Transaction::from_str(transaction.id().to_string().as_str())
                .change_context(Error::InvalidAleoTransactionId)?,
            transition: transition_id.clone(),
            destination_address: format!(
                "{:02X?}",
                &parsed_outputs.call_contract.destination_address
            ),
            destination_chain: ChainName::try_from(
                parsed_outputs.call_contract.destination_chain(),
            )
            .change_context(Error::InvalidChainName)?,
            source_address: Address::from_str(
                parsed_outputs.call_contract.sender.to_string().as_ref(),
            )
            .change_context(Error::InvalidSourceAddress)?,
            payload: parsed_outputs.payload,
        })
    }

    async fn transaction_receipt(
        &self,
        transaction_id: Transaction,
        transitions_id: Vec<Transition>,
    ) -> HashMap<Transition, Receipt> {
        let mut responces: HashMap<Transition, Receipt> = HashMap::new();

        let transaction = match self.client.get_transaction(&transaction_id).await {
            Ok(t) => t,
            Err(e) => {
                warn!(
                    "{e:?}, Failed to find transaction '{:?}'. The following transitions can not be vailidated: '{:?}'",
                    transaction_id,
                    transitions_id
                );

                for transition_id in transitions_id {
                    responces.insert(
                        transition_id.clone(),
                        Receipt::NotFound(
                            transaction_id.clone(),
                            transition_id,
                            Report::new(Error::HttpError),
                        ),
                    );
                }

                return responces;
            }
        };

        if transaction.execution().is_none() {
            warn!("Transaction '{:?}' is not an execution transaction. The following transitions can not be vailidated: '{:?}'",
                transaction_id,
                transitions_id
            );
            return responces;
        }

        // TODO: handle the option that is return by insert
        for transition_id in transitions_id {
            match self.transition_receipt(&transaction, &transition_id).await {
                Ok(receipt) => match responces.contains_key(&transition_id) {
                    false => {
                        responces.insert(transition_id, Receipt::Found(receipt));
                    }
                    true => todo!(),
                },
                Err(e) => match responces.contains_key(&transition_id) {
                    false => {
                        responces.insert(
                            transition_id.clone(),
                            Receipt::NotFound(transaction_id.clone(), transition_id, e),
                        );
                    }
                    true => todo!(),
                },
            }
        }

        responces
    }

    pub async fn transitions_receipts(
        &self,
        transactions: HashMap<Transaction, Vec<Transition>>,
    ) -> HashMap<Transition, Receipt> {
        let mut responces: HashMap<Transition, Receipt> = HashMap::new();
        /*
        The transaction-transition id

        The transition-id should be the function of the user

        [X] Get the transition and split the outputs
        [X] Match the call-contract data and the payload
        [X] Get the scm of the function call
        [X] Find all transitions with the same scm
        [X] Check that only one of them is the call-contract call
        [ ] Check that the CallContract has the same data as the function call
        */
        // for (transaction_id, transitions_id) in transactions {
        //     responces.extend(
        //         self.transaction_receipt(transaction_id, transitions_id)
        //             .await,
        //     );
        // }

        let stream = stream::iter(transactions)
            .map(|(transaction_id, transitions_id)| {
                self.transaction_receipt(transaction_id, transitions_id)
            })
            .buffer_unordered(10);

        tokio::pin!(stream);

        while let Some(value) = stream.next().await {
            responces.extend(value);
        }

        responces
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn mock_client() -> MockClientTrait {
        let mut mock_client = MockClientTrait::new();

        let transaction_id = "at1y098cyhn7m4c80h3g4s5d3js9ynzaahm0elpgx87tesgzlv0g59swdpeza";
        let mut expected_transitions: HashMap<Transaction, SnarkvmTransaction<CurrentNetwork>> =
            HashMap::new();
        let transaction_one = include_str!("../tests/aleo_transaction_at1y098cyhn7m4c80h3g4s5d3js9ynzaahm0elpgx87tesgzlv0g59swdpeza.json");
        let snark_tansaction =
            SnarkvmTransaction::<CurrentNetwork>::from_str(transaction_one).unwrap();
        let transaction = Transaction::from_str(transaction_id).unwrap();
        expected_transitions.insert(transaction, snark_tansaction);

        mock_client
            .expect_get_transaction()
            .returning(move |transaction| {
                Ok(expected_transitions.get(transaction).unwrap().clone())
            });

        mock_client
    }

    #[tokio::test]
    async fn foo_test() {
        let client = mock_client();
        let transaction_id = "at1y098cyhn7m4c80h3g4s5d3js9ynzaahm0elpgx87tesgzlv0g59swdpeza";
        let transision_id = "au1qazqhgn46lch9e5pjpqyvprueeaqlm66c5n9j6cuyxgyn7fkh5xqm38t2y";
        let transaction = Transaction::from_str(transaction_id).unwrap();
        // let res = client.get_transaction(&transaction).await;
        let client = ClientWrapper::new(client);
        let mut transisions = HashMap::new();
        transisions.insert(
            transaction,
            vec![Transition::from_str(transision_id).unwrap()],
        );
        let res = client.transitions_receipts(transisions).await;
        println!("{res:#?}");
    }

    // #[tokio::test]
    // async fn aleo_http() {
    //     // TODO: THIS TEST SHOULD BE REMOVE OR MADE TO WORK OFFLINE
    //     let aleo_http_client = AleoClient::new("https://api.explorer.provable.com/v1", "mainnet")
    //         .expect("Failed to create aleo http client");
    //
    //     let transactions: Result<Vec<Transaction>, _> = vec![
    //         Transaction::from_str("at1pvhkv0gt5qfnljte2nlav7u9rqrafgf04hzwkfu97sctynwghvqskfua6g"),
    //         Transaction::from_str("at1k70tqr8raf42mhny77gkj2r30g93eg4gsc78rkmsvs6ryeg4kyrq2r7enn"),
    //         Transaction::from_str("at1hz4p2qstrg24z25hvr988q0px5ch90p0tuda933m3g6ltgj3mvysy48py4"),
    //         Transaction::from_str("at1uu7vtv262r2eq79yhuhfg9qar87rn035qakxheqvkasdw6ne4yfqjk674s"),
    //         Transaction::from_str("at1c99efgfrm0ajcwxwlq9n533snyvcqtearehta5q7ruydja7f2gqq3dlfnd"),
    //         Transaction::from_str("at1222jayv9ldd6wj7r9j0z49ds7e3djkzk2upv0tl9csafcuncqvzqylr3m6"),
    //         Transaction::from_str("at1c0s58pqlq7pfhefjudjdl7xz0ujk45m2mrydsw7ygwh3ldd3syrsm4w93n"),
    //         Transaction::from_str("at1fe62jgs3xruvmjxzaxkycs8laqfc5204y76wkjpu8ucctcw55cyqjeamyd"),
    //         Transaction::from_str("at1ylsaru9y023e5862aehx5mw2xk9chlgergxcztn8lmrm3sc5fyxqpxnfek"),
    //         Transaction::from_str("at1683h72yrwk0sp4wq6qxdgkjazmn02y0w3uwltxpnhjc6k4uw4qyq6avlu8"),
    //         Transaction::from_str("at1yf67qmsnhrydqnv95llyymlu3s2u79yuut7fqv752ek2pdh24v9sjcchpa"),
    //         Transaction::from_str("at1eq0t8nfgersy5jmwx6t9dnul32n5qjte732d962s9x9l7txl05qqnwrnsu"),
    //         Transaction::from_str("at1pej4wqgvam3h49tjgy57tk3ukcym25826kzerup9kf3kc2afjcfqw7jhx9"),
    //         Transaction::from_str("at1e0e888pgcfw0dzgg45jtr4etnpj39a52hzv2t5etkk0waymfev9szyh8mk"),
    //     ]
    //     .into_iter()
    //     .collect();
    //     let transactions = transactions.expect("Failed to create transactions ids");
    //     // let transactions: HashSet<Transaction> = HashSet::from_iter(transactions);
    //
    //     // aleo_http_client.transitions_repsonses(transactions).await; // expect("Failed to validate transactions");
    //     //
    //     // let _fake_transactions: Result<Vec<Transaction>, _> = vec![Transaction::from_str(
    //     //     "at1agu5c94wxyp6sp3cmwtflr7u35zn3v8zfy2wm8wy0tjzm704q5xqcetqer",
    //     // )]
    //     // .into_iter()
    //     // .collect();
    // }
}
