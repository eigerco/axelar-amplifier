use std::collections::HashSet;
use std::convert::TryInto;

use async_trait::async_trait;
use axelar_wasm_std::voting::{PollId, Vote};
use connection_router_api::ChainName;
use cosmrs::cosmwasm::MsgExecuteContract;
use events::Error::EventTypeMismatch;
use events_derive::try_from;
use futures::future::join_all;
use serde::Deserialize;
use tokio::sync::watch::Receiver;
use tracing::info;
use voting_verifier::msg::ExecuteMsg;

use crate::event_processor::EventHandler;
use crate::handlers::errors::Error;
use crate::handlers::errors::Error::DeserializeEvent;
use crate::queue::queued_broadcaster::BroadcasterClient;
use crate::starknet::verifier::MessageVerifier;
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
    source_chain: ChainName,
    source_gateway_address: String,
    confirmation_height: u64,
    expires_at: u64,
    messages: Vec<Message>,
    participants: Vec<TMAddress>,
}

pub struct Handler<V, B>
where
    V: MessageVerifier,
    B: BroadcasterClient,
{
    worker: TMAddress,
    voting_verifier: TMAddress,
    msg_verifier: V,
    broadcast_client: B,
    latest_block_height: Receiver<u64>,
}

impl<V, B> Handler<V, B>
where
    V: MessageVerifier + Send + Sync,
    B: BroadcasterClient,
{
    pub fn new(
        worker: TMAddress,
        voting_verifier: TMAddress,
        msg_verifier: V,
        broadcast_client: B,
        latest_block_height: Receiver<u64>,
    ) -> Self {
        Self {
            worker,
            voting_verifier,
            msg_verifier,
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
    V: MessageVerifier + Send + Sync,
    B: BroadcasterClient + Send + Sync,
{
    type Err = Error;

    async fn handle(&self, event: &events::Event) -> Result<()> {
        let PollStartedEvent {
            contract_address,
            poll_id,
            source_chain,
            source_gateway_address: _,
            messages,
            expires_at,
            confirmation_height: _,
            participants,
        } = match event.try_into() as error_stack::Result<_, _> {
            Err(report) if matches!(report.current_context(), EventTypeMismatch(_)) => {
                return Ok(());
            }
            event => event.change_context(DeserializeEvent)?,
        };

        if self.voting_verifier != contract_address {
            println!("VOTING_VERIFIER!=CONTRACT_ADDRESS");
            return Ok(());
        }

        if !participants.contains(&self.worker) {
            println!("PARTICIPANT IS NOT THIS WORKER");
            return Ok(());
        }

        let latest_block_height = *self.latest_block_height.borrow();
        if latest_block_height >= expires_at {
            println!("POLL EXPIRED?");
            info!(poll_id = poll_id.to_string(), "skipping expired poll");
            return Ok(());
        }

        println!("ALL CHECKS PASSED");
        let tx_hashes: HashSet<_> = messages
            .iter()
            .map(|message| message.tx_id.as_str())
            .collect();

        let votes: Vec<Vote> = join_all(
            tx_hashes
                .into_iter()
                .map(|tx_hash| self.msg_verifier.verify(tx_hash)),
        )
        .await
        .into_iter()
        // TODO: Maybe log the errors (mostly with connection/serialization)?
        .filter_map(|v| v.ok())
        .collect();

        // Does not assume voting verifier emits unique tx ids.
        // RPC will throw an error if the input contains any duplicate, deduplicate tx
        // ids to avoid unnecessary failures.
        // let mut received_msgs_tx = HashSet::new();
        // let mut votes: Vec<Vote> = vec![];
        //
        // for msg in messages.iter() {
        //     votes.push(
        //         // Todo, maybe we can query all of them concurrently.
        //         match self.msg_verifier.verify(msg).await {
        //             Ok(_) => Vote::SucceededOnChain,
        //             Err(err) => {
        //                 println!("{:?} EEEEEER", err);
        //                 Vote::NotFound
        //             }
        //         },
        //     );
        // }
        self.broadcast_votes(poll_id, votes).await
    }
}

#[cfg(test)]
mod test {

    use axelar_wasm_std::nonempty;
    use base64::engine::general_purpose::STANDARD;
    use events::Event;
    use mockall::predicate::eq;
    use tendermint::abci;
    use tokio::sync::watch;
    use tokio::test as async_test;
    use voting_verifier::events::{PollMetadata, PollStarted, TxEventConfirmation};

    use super::*;
    use crate::queue::queued_broadcaster::MockBroadcasterClient;
    use crate::starknet::verifier::MockMessageVerifier;
    use crate::types::EVMAddress;
    use crate::PREFIX;

    #[async_test]
    async fn must_correctly_validate_messages() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        let msg = serde_json::to_vec(&ExecuteMsg::Vote {
            poll_id: "100".parse().unwrap(),
            votes: vec![Vote::SucceededOnChain, Vote::SucceededOnChain],
        })
        .expect("vote msg should serialize");

        let tx = MsgExecuteContract {
            sender: worker.as_ref().clone(),
            contract: voting_verifier.as_ref().clone(),
            msg,
            funds: vec![],
        };

        // Prepare the message verifier and the vote broadcaster
        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .once()
            .with(eq(tx))
            .returning(|_| Ok(()));
        let mut msg_verifier = MockMessageVerifier::new();
        msg_verifier
            .expect_verify()
            .times(2)
            .returning(|_| Ok(true));

        let event: Event = get_event(
            get_poll_started_event(participants(5, Some(worker.clone())), 100),
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, msg_verifier, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn must_skip_duplicated_tx() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        let msg = serde_json::to_vec(&ExecuteMsg::Vote {
            poll_id: "100".parse().unwrap(),
            // Only one vote expected, after deduplication.
            votes: vec![Vote::SucceededOnChain],
        })
        .expect("vote msg should serialize");

        let tx = MsgExecuteContract {
            sender: worker.as_ref().clone(),
            contract: voting_verifier.as_ref().clone(),
            msg,
            funds: vec![],
        };

        // Prepare the message verifier and the vote broadcaster
        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .once()
            .with(eq(tx))
            .returning(|_| Ok(()));
        let mut msg_verifier = MockMessageVerifier::new();
        msg_verifier
            .expect_verify()
            .once() // Only the first msg is verified, skipping the duplicated one.
            .returning(|_| Ok(true));

        let event: Event = get_event(
            get_poll_started_event_with_duplicates(participants(5, Some(worker.clone())), 100),
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, msg_verifier, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn ignores_poll_event_if_voting_verifier_address_not_match_event_address() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        // Prepare the message verifier and the vote broadcaster
        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .never();

        let mut msg_verifier = MockMessageVerifier::new();
        msg_verifier.expect_verify().never();

        let event: Event = get_event(
            get_poll_started_event(participants(5, Some(worker.clone())), 100),
            &TMAddress::random(PREFIX), // A different, unexpected address comes from the event.
        );

        let handler =
            super::Handler::new(worker, voting_verifier, msg_verifier, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn ignores_poll_event_if_worker_not_part_of_participants() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration - 1);

        // Prepare the message verifier and the vote broadcaster
        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .never();

        let mut msg_verifier = MockMessageVerifier::new();
        msg_verifier.expect_verify().never();

        let event: Event = get_event(
            get_poll_started_event(participants(5, None), 100), /* This worker is not in
                                                                 * participant set. So will skip
                                                                 * the event. */
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, msg_verifier, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
    }

    #[async_test]
    async fn ignores_expired_poll_event() {
        // Setup the context
        let voting_verifier = TMAddress::random(PREFIX);
        let worker = TMAddress::random(PREFIX);
        let expiration = 100u64;
        let (_, rx) = watch::channel(expiration); // expired !

        // Prepare the message verifier and the vote broadcaster
        let mut broadcast_client = MockBroadcasterClient::new();
        broadcast_client
            .expect_broadcast::<MsgExecuteContract>()
            .never();

        let mut msg_verifier = MockMessageVerifier::new();
        msg_verifier.expect_verify().never();

        let event: Event = get_event(
            get_poll_started_event(participants(5, Some(worker.clone())), 100),
            &voting_verifier,
        );

        let handler =
            super::Handler::new(worker, voting_verifier, msg_verifier, broadcast_client, rx);

        handler.handle(&event).await.unwrap();
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

    fn get_poll_started_event(participants: Vec<TMAddress>, expires_at: u64) -> PollStarted {
        get_poll_started_event_with_source_chain(participants, expires_at, "starknet")
    }

    fn get_poll_started_event_with_source_chain(
        participants: Vec<TMAddress>,
        expires_at: u64,
        source_chain: &str,
    ) -> PollStarted {
        PollStarted::Messages {
            metadata: PollMetadata {
                poll_id: "100".parse().unwrap(),
                source_chain: source_chain.parse().unwrap(),
                source_gateway_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                confirmation_height: 15,
                expires_at,
                participants: participants
                    .into_iter()
                    .map(|addr| cosmwasm_std::Addr::unchecked(addr.to_string()))
                    .collect(),
            },
            messages: vec![
                TxEventConfirmation {
                    tx_id: format!("0x{:x}", Hash::random()).parse().unwrap(),
                    event_index: 10,
                    source_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    payload_hash: Hash::random().to_fixed_bytes(),
                },
                TxEventConfirmation {
                    tx_id: format!("0x{:x}", Hash::random()).parse().unwrap(),
                    event_index: 11,
                    source_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    payload_hash: Hash::random().to_fixed_bytes(),
                },
            ],
        }
    }

    fn get_poll_started_event_with_duplicates(
        participants: Vec<TMAddress>,
        expires_at: u64,
    ) -> PollStarted {
        let tx_id: nonempty::String = format!("0x{:x}", Hash::random()).parse().unwrap();
        PollStarted::Messages {
            metadata: PollMetadata {
                poll_id: "100".parse().unwrap(),
                source_chain: "starknet".parse().unwrap(),
                source_gateway_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                confirmation_height: 15,
                expires_at,
                participants: participants
                    .into_iter()
                    .map(|addr| cosmwasm_std::Addr::unchecked(addr.to_string()))
                    .collect(),
            },
            messages: vec![
                TxEventConfirmation {
                    tx_id: tx_id.clone(),
                    event_index: 10,
                    source_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    payload_hash: Hash::random().to_fixed_bytes(),
                },
                TxEventConfirmation {
                    tx_id,
                    event_index: 10,
                    source_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: format!("0x{:x}", EVMAddress::random()).parse().unwrap(),
                    payload_hash: Hash::random().to_fixed_bytes(),
                },
            ],
        }
    }

    fn participants(n: u8, worker: Option<TMAddress>) -> Vec<TMAddress> {
        (0..n)
            .into_iter()
            .map(|_| TMAddress::random(PREFIX))
            .chain(worker.into_iter())
            .collect()
    }
}
