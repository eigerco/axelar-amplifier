use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use aleo_types::address::Address as AleoAddress;
use aleo_types::program::Program;
use aleo_types::transition::Transition;
use async_trait::async_trait;
use axelar_wasm_std::voting::{PollId, Vote};
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::tx::Msg;
use events::Error::EventTypeMismatch;
use events::Event;
use events_derive::try_from;
use futures::stream::{self, StreamExt};
use multisig::verifier_set::VerifierSet;
use prost_types::Any;
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use tokio::sync::watch::Receiver;
use tracing::{debug, info, info_span};
use valuable::Valuable;
use voting_verifier::msg::ExecuteMsg;

use crate::aleo::http_client::{ClientTrait as AleoClientTrait, Driver, Receipt};
use crate::aleo::verifier::verify_verifier_set;
use crate::event_processor::EventHandler;
use crate::handlers::errors::Error;
use crate::handlers::errors::Error::DeserializeEvent;
use crate::types::{Hash, TMAddress};

type Result<T> = error_stack::Result<T, Error>;

#[derive(Deserialize, Debug)]
pub struct VerifierSetConfirmation {
    pub tx_id: Transition,
    pub verifier_set: VerifierSet,
}

#[derive(Deserialize, Debug)]
#[try_from("wasm-verifier_set_poll_started")]
struct PollStartedEvent {
    verifier_set: VerifierSetConfirmation,
    poll_id: PollId,
    source_chain: ChainName,
    expires_at: u64,
    participants: Vec<TMAddress>,
}

#[derive(Clone)]
pub struct Handler<C: AleoClientTrait> {
    verifier: TMAddress,
    voting_verifier_contract: TMAddress,
    http_client: C,
    latest_block_height: Receiver<u64>,
    chain: ChainName,
    verifier_set_contract: String,
}

impl<C> Handler<C>
where
    C: AleoClientTrait + Send + Sync,
{
    pub fn new(
        verifier: TMAddress,
        voting_verifier_contract: TMAddress,
        chain: ChainName,
        aleo_client: C,
        latest_block_height: Receiver<u64>,
        gateway_contract: String,
    ) -> Self {
        Self {
            verifier,
            voting_verifier_contract,
            http_client: aleo_client,
            latest_block_height,
            chain,
            verifier_set_contract: gateway_contract,
        }
    }

    fn vote_msg(&self, poll_id: PollId, vote: Vote) -> MsgExecuteContract {
        MsgExecuteContract {
            sender: self.verifier.as_ref().clone(),
            contract: self.voting_verifier_contract.as_ref().clone(),
            msg: serde_json::to_vec(&ExecuteMsg::Vote {
                poll_id,
                votes: vec![vote],
            })
            .expect("vote msg should serialize"),
            funds: vec![],
        }
    }
}

async fn fetch_transition_receipt<C>(
    http_client: &C,
    program: Program,
    id: Transition,
) -> (Transition, Receipt)
where
    C: AleoClientTrait + Send + Sync + 'static,
{
    let driver = Driver::new(http_client, program);

    let receipt = async {
        driver
            .get_transaction_id(id.clone())
            .await?
            .get_transaction()
            .await?
            .get_transition()? // TODO: check if this should go out of async
            .check_signer_rotation()
    }
    .await;

    match receipt {
        Ok(receipt) => (id, receipt),
        Err(e) => (id.clone(), Receipt::NotFound(id, e)),
    }
}

#[async_trait]
impl<C> EventHandler for Handler<C>
where
    C: AleoClientTrait + Send + Sync + 'static,
{
    type Err = Error;

    #[tracing::instrument(skip(self, event))]
    async fn handle(&self, event: &Event) -> error_stack::Result<Vec<Any>, Self::Err> {
        debug!("event: {event:?}");
        if !event.is_from_contract(self.voting_verifier_contract.as_ref()) {
            return Ok(vec![]);
        }

        let PollStartedEvent {
            poll_id,
            source_chain,
            expires_at,
            participants,
            verifier_set,
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

        // Transition IDs on Aleo chain
        let transition = verifier_set.tx_id;

        let program = Program::from_str(self.verifier_set_contract.as_str()).unwrap();

        let receipt =
            fetch_transition_receipt(&self.http_client, program, transition.clone()).await;

        let poll_id_str: String = poll_id.into();
        let source_chain_str: String = source_chain.into();
        let vote = info_span!(
            "verify messages from an Aleo chain",
            poll_id = poll_id_str,
            source_chain = source_chain_str,
            message_ids = transition.to_string(),
        )
        .in_scope(|| {
            info!("ready to verify messages in poll");

            let vote = verify_verifier_set(&receipt.1);
            info!(
                vote = ?vote,
                "ready to vote for messages in poll"
            );

            vote
        });

        Ok(vec![self
            .vote_msg(poll_id, vote)
            .into_any()
            .expect("vote msg should serialize")])
    }
}
