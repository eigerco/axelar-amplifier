//! Module responsible for handling verification of verifier set changes on Starknet.
//! It processes events related to verifier set updates, verifies them against the Starknet chain,
//! and manages the voting process for confirming these changes.

use std::convert::TryInto;

use async_trait::async_trait;
use axelar_wasm_std::voting::{PollId, Vote};
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::tx::Msg;
use cosmrs::Any;
use error_stack::ResultExt;
use events::Error::EventTypeMismatch;
use events_derive::try_from;
use multisig::verifier_set::VerifierSet;
use serde::Deserialize;
use starknet_core::types::FieldElement;
use tokio::sync::watch::Receiver;
use tracing::{info, info_span};
use valuable::Valuable;
use voting_verifier::msg::ExecuteMsg;

use crate::event_processor::EventHandler;
use crate::handlers::errors::Error;
use crate::starknet::json_rpc::StarknetClient;
use crate::starknet::verifier::verify_verifier_set;
use crate::types::{Hash, TMAddress};

#[derive(Deserialize, Debug)]
pub struct VerifierSetConfirmation {
    pub tx_hash: FieldElement,
    pub event_index: u32,
    pub verifier_set: VerifierSet,
}

#[derive(Deserialize, Debug)]
#[try_from("wasm-verifier_set_poll_started")]
struct PollStartedEvent {
    poll_id: PollId,
    source_gateway_address: String,
    verifier_set: VerifierSetConfirmation,
    participants: Vec<TMAddress>,
    expires_at: u64,
}

pub struct Handler<C>
where
    C: StarknetClient + Send + Sync,
{
    verifier: TMAddress,
    voting_verifier_contract: TMAddress,
    rpc_client: C,
    latest_block_height: Receiver<u64>,
}

impl<C> Handler<C>
where
    C: StarknetClient + Send + Sync,
{
    pub fn new(
        verifier: TMAddress,
        voting_verifier_contract: TMAddress,
        rpc_client: C,
        latest_block_height: Receiver<u64>,
    ) -> Self {
        Self {
            verifier,
            voting_verifier_contract,
            rpc_client,
            latest_block_height,
        }
    }
}

#[async_trait]
impl EventHandler for Handler<C>
where
    C: StarknetClient + Send + Sync,
{
    type Err = Error;

    async fn handle(&self, event: &Event) -> Result<Vec<Any>, Self::Err> {
        if !event.is_from_contract(self.voting_verifier_contract.as_ref()) {
            return Ok(vec![]);
        }

        let PollStartedEvent {
            poll_id,
            source_gateway_address,
            verifier_set,
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

        // FIXME: the rpc client default to CallContractEvent, it has to be extended
        let event_or_not_event_thats_the_question = self
            .rpc_client
            .get_event_by_hash(verifier_set.tx_hash)
            .await?;

        let vote = info_span!(
            "verify a new verifier set",
            poll_id = poll_id.to_string(),
            id = verifier_set.message_id.to_string(),
        )
        .in_scope(|| {
            info!("ready to verify verifier set in poll",);

            let vote = transaction_response.map_or(Vote::NotFound, |tx_receipt| {
                verify_verifier_set(&source_gateway_address, &tx_receipt, &verifier_set)
            });

            info!(
                vote = vote.as_value(),
                "ready to vote for a new verifier set in poll"
            );

            vote
        });

        Ok(vec![self
            .vote_msg(poll_id, vote) // TODO: check if this shouldn't be a vec
            .into_any()
            .expect("vote msg should serialize")])
    }
}

// TODO: add tests
