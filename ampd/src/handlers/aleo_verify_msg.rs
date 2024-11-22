use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use aleo_types::address::Address;
use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use async_trait::async_trait;
use axelar_wasm_std::voting::{PollId, Vote};
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::tx::Msg;
use events::Error::EventTypeMismatch;
use events::Event;
use events_derive::try_from;
use futures::stream::{self, StreamExt};
use prost_types::Any;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use tokio::sync::watch::Receiver;
use tracing::{info, info_span};
use valuable::Valuable;
use voting_verifier::msg::ExecuteMsg;

use crate::aleo::http_client::{
    ClientTrait as AleoClientTrait, ClientWrapper as AleoClientWrapper, Receipt,
};
use crate::event_processor::EventHandler;
use crate::handlers::errors::Error;
use crate::handlers::errors::Error::DeserializeEvent;
use crate::types::{Hash, TMAddress};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub transition_id: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: Address,
    pub payload_hash: Hash,
}

#[derive(Deserialize, Debug)]
#[try_from("wasm-messages_poll_started")]
struct PollStartedEvent {
    poll_id: PollId,
    source_chain: ChainName,
    source_gateway_address: Address,
    expires_at: u64,
    messages: Vec<Message>,
    participants: Vec<TMAddress>,
}

#[derive(Clone)]
pub struct Handler<C: AleoClientTrait> {
    verifier: TMAddress,
    voting_verifier_contract: TMAddress,
    http_client: C,
    latest_block_height: Receiver<u64>,
    chain: ChainName,
    gateway_contract: String,
}

impl<C> Handler<C>
where
    C: AleoClientTrait + Send + Sync,
{
    pub fn new(
        verifier: TMAddress,
        voting_verifier_contract: TMAddress,
        aleo_client: C,
        latest_block_height: Receiver<u64>,
        gateway_contract: String,
    ) -> Self {
        Self {
            verifier,
            voting_verifier_contract,
            http_client: aleo_client,
            latest_block_height,
            chain: ChainName::from_str("aleo").unwrap(),
            gateway_contract,
        }
    }

    fn vote_msg(&self, poll_id: PollId, votes: Vec<Vote>) -> MsgExecuteContract {
        MsgExecuteContract {
            sender: self.verifier.as_ref().clone(),
            contract: self.voting_verifier_contract.as_ref().clone(),
            msg: serde_json::to_vec(&ExecuteMsg::Vote { poll_id, votes })
                .expect("vote msg should serialize"),
            funds: vec![],
        }
    }
}

#[async_trait]
impl<C> EventHandler for Handler<C>
where
    C: AleoClientTrait + Send + Sync + Clone + 'static,
{
    type Err = Error;

    async fn handle(&self, event: &Event) -> error_stack::Result<Vec<Any>, Self::Err> {
        let http_client = AleoClientWrapper::new(self.http_client.clone());

        if !event.is_from_contract(self.voting_verifier_contract.as_ref()) {
            return Ok(vec![]);
        }

        let PollStartedEvent {
            poll_id,
            source_chain,
            source_gateway_address,
            messages,
            expires_at,
            participants,
        } = match event.try_into() as error_stack::Result<_, _> {
            Err(report) if matches!(report.current_context(), EventTypeMismatch(_)) => {
                return Ok(vec![])
            }
            event => event.change_context(DeserializeEvent)?,
        };

        if self.chain != source_chain {
            return Ok(vec![]);
        }

        if !participants.contains(&self.verifier) {
            return Ok(vec![]);
        }

        if *self.latest_block_height.borrow() >= expires_at {
            info!(poll_id = poll_id.to_string(), "skipping expired poll");
            return Ok(vec![]);
        }

        let transitions: HashSet<Transition> =
            messages.iter().map(|m| m.transition_id.clone()).collect();

        let transition_receipts: HashMap<_, _> = stream::iter(transitions)
            .map(|id| async {
                match http_client
                    .transition_receipt(&id, self.gateway_contract.as_str())
                    .await
                {
                    Ok(recipt) => (id, recipt),
                    Err(e) => (id.clone(), Receipt::NotFound(id, e)),
                }
            })
            .buffer_unordered(10)
            .collect()
            .await;

        let poll_id_str: String = poll_id.into();
        let source_chain_str: String = source_chain.into();
        let votes = info_span!(
            "verify messages from an Aleo chain",
            poll_id = poll_id_str,
            source_chain = source_chain_str,
            message_ids = messages
                .iter()
                .map(|msg| { format!("{}", msg.transition_id) })
                .collect::<Vec<String>>()
                .as_value(),
        )
        .in_scope(|| {
            info!("ready to verify messages in poll",);

            let votes: Vec<_> = messages
                .iter()
                .map(|msg| {
                    transition_receipts
                        .get(&msg.transition_id)
                        .map_or(Vote::NotFound, |tx_receipt| {
                            crate::aleo::verifier::verify_message(tx_receipt, msg)
                        })
                })
                .collect();
            info!(
                votes = votes.as_value(),
                "ready to vote for messages in poll"
            );

            votes
        });

        Ok(vec![self
            .vote_msg(poll_id, votes)
            .into_any()
            .expect("vote msg should serialize")])
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use axelar_wasm_std::nonempty::String;
    use router_api::Address;
    use voting_verifier::events::{PollMetadata, PollStarted, TxEventConfirmation};

    use crate::types::TMAddress;

    fn poll_started_event(participants: Vec<TMAddress>, expires_at: u64) -> PollStarted {
        let message_id = "at18c83pwjlvvjpdk95pudngzxqydvq92np206njcyppgndjalujsrshjn48j";

        PollStarted::Messages {
            metadata: PollMetadata {
                poll_id: "100".parse().unwrap(),
                source_chain: "aleo".parse().unwrap(),
                source_gateway_address: "vzevxifdoj.aleo"
                    .parse()
                    .unwrap(),
                confirmation_height: 15,
                expires_at,
                participants: participants
                    .into_iter()
                    .map(|addr| cosmwasm_std::Addr::unchecked(addr.to_string()))
                    .collect(),
            },
            #[allow(deprecated)] // TODO: The below events use the deprecated tx_id and event_index fields. Remove this attribute when those fields are removed
            messages: vec![
                TxEventConfirmation {
                    tx_id: String::from_str("deprecated").unwrap(),
                    event_index: 5u32,
                    message_id: message_id.to_string().try_into().unwrap(),
                    source_address: Address::from_str("vzevxifdoj.aleo").unwrap(),
                    destination_chain: "ethereum".parse().unwrap(),
                    destination_address: Address::from_str("0x1D1479C185d32EB90533a08b36B3CFa5F84A0E6B").unwrap(),
                    payload_hash: [0u8; 32],
                },
            ],
        }
    }
}
