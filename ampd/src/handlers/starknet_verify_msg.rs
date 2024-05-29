use std::collections::HashMap;
use std::convert::TryInto;

use async_trait::async_trait;
use axelar_wasm_std::voting::{PollId, Vote};
use cosmrs::cosmwasm::MsgExecuteContract;
use error_stack::{FutureExt, ResultExt};
use events::Error::EventTypeMismatch;
use events_derive::try_from;
use futures::future::try_join_all;
use itertools::Itertools;
use serde::Deserialize;
use tokio::sync::watch::Receiver;
use tracing::info;
use voting_verifier::msg::ExecuteMsg;

use crate::event_processor::EventHandler;
use crate::handlers::errors::Error;
use crate::handlers::errors::Error::DeserializeEvent;
use crate::queue::queued_broadcaster::BroadcasterClient;
use crate::starknet::events::contract_call::ContractCallEvent;
use crate::starknet::json_rpc::StarknetClient;
use crate::starknet::verifier::verify_msg;
use crate::types::{Hash, TMAddress};

type Result<T> = error_stack::Result<T, Error>;

#[derive(Deserialize, Debug)]
pub struct Message {
    pub tx_id: String,
    pub event_index: u64,
    pub destination_address: String,
    pub destination_chain: String,
    pub source_address: String,
    pub payload_hash: Hash,
}

#[derive(Deserialize, Debug)]
#[try_from("wasm-messages_poll_started")]
struct PollStartedEvent {
    #[serde(rename = "_contract_address")]
    contract_address: TMAddress,
    poll_id: PollId,
    source_gateway_address: String,
    expires_at: u64,
    messages: Vec<Message>,
    participants: Vec<TMAddress>,
}

pub struct Handler<C, B>
where
    C: StarknetClient,
    B: BroadcasterClient,
{
    worker: TMAddress,
    voting_verifier: TMAddress,
    rpc_client: C,
    broadcast_client: B,
    latest_block_height: Receiver<u64>,
}

impl<C, B> Handler<C, B>
where
    C: StarknetClient + Send + Sync,
    B: BroadcasterClient,
{
    pub fn new(
        worker: TMAddress,
        voting_verifier: TMAddress,
        rpc_client: C,
        broadcast_client: B,
        latest_block_height: Receiver<u64>,
    ) -> Self {
        Self {
            worker,
            voting_verifier,
            rpc_client,
            broadcast_client,
            latest_block_height,
        }
    }

    async fn broadcast_votes(&self, poll_id: PollId, votes: Vec<Vote>) -> Result<()> {
        let msg = serde_json::to_vec(&ExecuteMsg::Vote { poll_id, votes })
            .expect("vote msg should serialize");
        let tx = MsgExecuteContract {
            sender: self.worker.as_ref().clone(),
            contract: self.voting_verifier.as_ref().clone(),
            msg,
            funds: vec![],
        };

        self.broadcast_client
            .broadcast(tx)
            .await
            .change_context(Error::Broadcaster)
    }
}

#[async_trait]
impl<V, B> EventHandler for Handler<V, B>
where
    V: StarknetClient + Send + Sync,
    B: BroadcasterClient + Send + Sync,
{
    type Err = Error;

    async fn handle(&self, event: &events::Event) -> Result<()> {
        let PollStartedEvent {
            poll_id,
            source_gateway_address,
            messages,
            participants,
            expires_at,
            contract_address,
            ..
        } = match event.try_into() as error_stack::Result<_, _> {
            Err(report) if matches!(report.current_context(), EventTypeMismatch(_)) => {
                return Ok(());
            }
            event => event.change_context(DeserializeEvent)?,
        };

        if self.voting_verifier != contract_address {
            return Ok(());
        }

        if !participants.contains(&self.worker) {
            return Ok(());
        }

        let latest_block_height = *self.latest_block_height.borrow();
        if latest_block_height >= expires_at {
            info!(poll_id = poll_id.to_string(), "skipping expired poll");
            return Ok(());
        }

        let unique_msgs = messages
            .iter()
            .unique_by(|msg| &msg.tx_id)
            .collect::<Vec<_>>();
        //
        // key is the tx_hash of the tx holding the event
        let events: HashMap<String, ContractCallEvent> = try_join_all(
            unique_msgs
                .iter()
                .map(|msg| self.rpc_client.get_event_by_hash(msg.tx_id.as_str())),
        )
        .change_context(Error::TxReceipts)
        .await?
        .into_iter()
        .flatten()
        .collect();

        let mut votes = vec![];
        for msg in unique_msgs {
            if !events.contains_key(&msg.tx_id) {
                votes.push(Vote::NotFound);
                continue;
            }
            votes.push(verify_msg(
                events.get(&msg.tx_id).unwrap(), // safe to unwrap, because of previous check
                msg,
                &source_gateway_address,
            ));
        }

        self.broadcast_votes(poll_id, votes).await
    }
}

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use ethers::types::H256;
    use events::Event;
    use mockall::predicate::eq;
    use tendermint::abci;
    use tokio::sync::watch;
    use tokio::test as async_test;
    use voting_verifier::events::{PollMetadata, PollStarted, TxEventConfirmation};

    use super::*;
    use crate::queue::queued_broadcaster::MockBroadcasterClient;
    use crate::starknet::json_rpc::MockStarknetClient;
    use crate::PREFIX;

    #[async_test]
    async fn should_correctly_validate_messages() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        let vote_broadcast_msg = serde_json::to_vec(&ExecuteMsg::Vote {
            poll_id: "100".parse().unwrap(),
            votes: vec![Vote::SucceededOnChain, Vote::NotFound],
        })
        .expect("vote msg should serialize");

        let tx = MsgExecuteContract {
            sender: worker.as_ref().clone(),
            contract: voting_verifier.as_ref().clone(),
            msg: vote_broadcast_msg,
            funds: vec![],
        };

        // Prepare the rpc client, which fetches the event and the vote broadcaster
        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .once()
            .with(eq(tx))
            .returning(|_| Ok(()));

        let mut rpc_client = MockStarknetClient::new();
        rpc_client.expect_get_event_by_hash().returning(|_| {
            Ok(Some((
                String::from("txhash123"),
                ContractCallEvent {
                    from_contract_addr: String::from("source_gw_addr"),
                    destination_address: String::from("destination_address"),
                    destination_chain: String::from("ethereum"),
                    source_address: String::from("source_address"),
                    payload_hash: H256::from_slice(&[
                        28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123,
                        86, 217, 81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
                    ]),
                },
            )))
        });

        let event: Event = get_event(
            get_poll_started_event_with_two_msgs(participants(5, Some(worker.clone())), 100),
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, rpc_client, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn should_skip_duplicate_messages() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        let vote_broadcast_msg = serde_json::to_vec(&ExecuteMsg::Vote {
            poll_id: "100".parse().unwrap(),
            votes: vec![Vote::SucceededOnChain],
        })
        .expect("vote msg should serialize");

        let tx = MsgExecuteContract {
            sender: worker.as_ref().clone(),
            contract: voting_verifier.as_ref().clone(),
            msg: vote_broadcast_msg,
            funds: vec![],
        };

        // Prepare the rpc client, which fetches the event and the vote broadcaster
        let mut rpc_client = MockStarknetClient::new();
        rpc_client
            .expect_get_event_by_hash()
            .once()
            .with(eq("txhash123"))
            .returning(|_| {
                Ok(Some((
                    String::from("txhash123"),
                    ContractCallEvent {
                        from_contract_addr: String::from("source_gw_addr"),
                        destination_address: String::from("destination_address"),
                        destination_chain: String::from("ethereum"),
                        source_address: String::from("source_address"),
                        payload_hash: H256::from_slice(&[
                            28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40,
                            123, 86, 217, 81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109,
                            234, 200,
                        ]),
                    },
                )))
            });

        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .once()
            .with(eq(tx))
            .returning(|_| Ok(()));

        let event: Event = get_event(
            get_poll_started_event_with_duplicate_msgs(participants(5, Some(worker.clone())), 100),
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, rpc_client, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn should_skip_wrong_verifier_address() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        // Prepare the rpc client, which fetches the event and the vote broadcaster
        let mut rpc_client = MockStarknetClient::new();
        rpc_client.expect_get_event_by_hash().times(0);

        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .times(0);

        let event: Event = get_event(
            get_poll_started_event_with_duplicate_msgs(participants(5, Some(worker.clone())), 100),
            &TMAddress::random(PREFIX), // some other random address
        );

        let handler =
            super::Handler::new(worker, voting_verifier, rpc_client, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn should_skip_non_participating_worker() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        // Prepare the rpc client, which fetches the event and the vote broadcaster
        let mut rpc_client = MockStarknetClient::new();
        rpc_client.expect_get_event_by_hash().times(0);

        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .times(0);

        let event: Event = get_event(
            // woker is not in participat set
            get_poll_started_event_with_duplicate_msgs(participants(5, None), 100),
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, rpc_client, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn should_skip_expired_poll_event() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration); // expired!

        // Prepare the rpc client, which fetches the event and the vote broadcaster
        let mut rpc_client = MockStarknetClient::new();
        rpc_client.expect_get_event_by_hash().times(0);

        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .times(0);

        let event: Event = get_event(
            // woker is not in participat set
            get_poll_started_event_with_duplicate_msgs(participants(5, None), 100),
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, rpc_client, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    fn participants(n: u8, worker: Option<TMAddress>) -> Vec<TMAddress> {
        (0..n)
            .map(|_| TMAddress::random(PREFIX))
            .chain(worker)
            .collect()
    }

    fn get_event(event: impl Into<cosmwasm_std::Event>, contract_address: &TMAddress) -> Event {
        let mut event: cosmwasm_std::Event = event.into();

        event.ty = format!("wasm-{}", event.ty);
        event = event.add_attribute("_contract_address", contract_address.to_string());

        abci::Event::new(
            event.ty,
            event
                .attributes
                .into_iter()
                .map(|cosmwasm_std::Attribute { key, value }| {
                    (STANDARD.encode(key), STANDARD.encode(value))
                }),
        )
        .try_into()
        .unwrap()
    }

    fn get_poll_started_event_with_two_msgs(
        participants: Vec<TMAddress>,
        expires_at: u64,
    ) -> PollStarted {
        PollStarted::Messages {
            metadata: PollMetadata {
                poll_id: "100".parse().unwrap(),
                source_chain: "starknet".parse().unwrap(),
                source_gateway_address: "source_gw_addr".parse().unwrap(),
                confirmation_height: 15,
                expires_at,
                participants: participants
                    .into_iter()
                    .map(|addr| cosmwasm_std::Addr::unchecked(addr.to_string()))
                    .collect(),
            },
            messages: vec![
                TxEventConfirmation {
                    tx_id: "txhash123".parse().unwrap(),
                    event_index: 0,
                    source_address: "source_address".parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: "destination_address".parse().unwrap(),
                    payload_hash: H256::from_slice(&[
                        // keccak256("hello")
                        28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123,
                        86, 217, 81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
                    ])
                    .into(),
                },
                TxEventConfirmation {
                    tx_id: "txhash456".parse().unwrap(),
                    event_index: 1,
                    source_address: "source_address".parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: "destination_address".parse().unwrap(),
                    payload_hash: H256::from_slice(&[
                        // keccak256("hello")
                        28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123,
                        86, 217, 81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
                    ])
                    .into(),
                },
            ],
        }
    }

    fn get_poll_started_event_with_duplicate_msgs(
        participants: Vec<TMAddress>,
        expires_at: u64,
    ) -> PollStarted {
        PollStarted::Messages {
            metadata: PollMetadata {
                poll_id: "100".parse().unwrap(),
                source_chain: "starknet".parse().unwrap(),
                source_gateway_address: "source_gw_addr".parse().unwrap(),
                confirmation_height: 15,
                expires_at,
                participants: participants
                    .into_iter()
                    .map(|addr| cosmwasm_std::Addr::unchecked(addr.to_string()))
                    .collect(),
            },
            messages: vec![
                TxEventConfirmation {
                    tx_id: "txhash123".parse().unwrap(),
                    event_index: 0,
                    source_address: "source_address".parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: "destination_address".parse().unwrap(),
                    payload_hash: H256::from_slice(&[
                        // keccak256("hello")
                        28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123,
                        86, 217, 81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
                    ])
                    .into(),
                },
                TxEventConfirmation {
                    tx_id: "txhash123".parse().unwrap(),
                    event_index: 1,
                    source_address: "source_address".parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: "destination_address".parse().unwrap(),
                    payload_hash: H256::from_slice(&[
                        // keccak256("hello")
                        28u8, 138, 255, 149, 6, 133, 194, 237, 75, 195, 23, 79, 52, 114, 40, 123,
                        86, 217, 81, 123, 156, 148, 129, 39, 49, 154, 9, 167, 163, 109, 234, 200,
                    ])
                    .into(),
                },
            ],
        }
    }
}
