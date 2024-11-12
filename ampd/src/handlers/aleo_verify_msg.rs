use std::collections::HashSet;

use aleo_types::{Address, Transaction, Transition};
use async_trait::async_trait;
use axelar_wasm_std::voting::{PollId, Vote};
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::proto::tendermint::blocksync::message;
use events::Error::EventTypeMismatch;
use events::Event;
use events_derive::try_from;
use prost_types::Any;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use tokio::sync::watch::Receiver;
use tracing::{info, info_span};
use valuable::Valuable;
use voting_verifier::msg::ExecuteMsg;

use crate::aleo::http_client::Client;
use crate::event_processor::EventHandler;
use crate::handlers::errors::Error;
use crate::handlers::errors::Error::DeserializeEvent;
use crate::types::{Hash, TMAddress};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub message_id: Transaction,
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

pub struct Handler {
    verifier: TMAddress,
    voting_verifier_contract: TMAddress,
    http_client: Client,
    latest_block_height: Receiver<u64>,
}

impl Handler {
    pub fn new(
        verifier: TMAddress,
        voting_verifier_contract: TMAddress,
        http_client: Client,
        latest_block_height: Receiver<u64>,
    ) -> Self {
        Self {
            verifier,
            voting_verifier_contract,
            http_client,
            latest_block_height,
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
impl EventHandler for Handler {
    type Err = Error;

    async fn handle(&self, event: &Event) -> error_stack::Result<Vec<Any>, Self::Err> {
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

        if !participants.contains(&self.verifier) {
            return Ok(vec![]);
        }

        if *self.latest_block_height.borrow() >= expires_at {
            info!(poll_id = poll_id.to_string(), "skipping expired poll");
            return Ok(vec![]);
        }

        let transactions: HashSet<_> = messages
            .iter()
            .map(|message| message.message_id.clone())
            .collect();

        let transaction_responses = self.http_client.transaction_responses(transactions).await;

        // let message_ids = messages
        //     .iter()
        //     .map(|message| message.message_id.to_string())
        //     .collect::<Vec<_>>();

        // let votes = info_span!(
        //     "verify messages in poll",
        //     poll_id = poll_id.to_string(),
        //     source_chain = source_chain.to_string(),
        //     message_ids = message_ids.as_value()
        // )
        // .in_scope(|| {
        //     info!("ready to verify messages in poll",);
        //
        //     let votes: Vec<_> = messages
        //         .iter()
        //         .map(|msg| {
        //             transaction_responses
        //                 .get(&msg.message_id.tx_hash_as_hex_no_prefix().to_string())
        //                 .map_or(Vote::NotFound, |tx_response| {
        //                     verify_message(&source_gateway_address, tx_response, msg)
        //                 })
        //         })
        //         .collect();
        //     info!(
        //         votes = votes.as_value(),
        //         "ready to vote for messages in poll"
        //     );
        //
        //     votes
        // });
        //
        // Ok(vec![self
        //     .vote_msg(poll_id, votes)
        //     .into_any()
        //     .expect("vote msg should serialize")])
        todo!()
    }
}
