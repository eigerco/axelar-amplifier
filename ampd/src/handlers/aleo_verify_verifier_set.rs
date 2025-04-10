use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use aleo_gateway::WeightedSigners;
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
    pub message_id: Transition,
    pub verifier_set: VerifierSet,
}

#[derive(Deserialize, Debug)]
#[try_from("wasm-verifier_set_poll_started")]
struct PollStartedEvent {
    verifier_set: VerifierSetConfirmation,
    poll_id: PollId,
    source_chain: ChainName,
    source_gateway_address: Program,
    expires_at: u64,
    confirmation_height: u64,
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
            source_gateway_address,
            expires_at,
            confirmation_height,
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
        let transition = &verifier_set.message_id;

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

            let vote = verify_verifier_set(&receipt.1, &verifier_set);
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

#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::str::FromStr;

    use aleo_types::transition::Transition;
    use axelar_wasm_std::msg_id::HexTxHashAndEventIndex;
    use axelar_wasm_std::voting::PollId;
    use cosmwasm_std::Addr;
    use error_stack::Report;
    use ethers_core::types::H256;
    use ethers_providers::ProviderError;
    use events::Event;
    use multisig::key::KeyType;
    use multisig::test::common::{aleo_schnorr_test_data, build_verifier_set, ecdsa_test_data};
    use multisig::verifier_set::VerifierSet;
    use router_api::ChainName;
    use tokio::sync::watch;
    use tokio::test as async_test;
    use voting_verifier::events::{PollMetadata, PollStarted, VerifierSetConfirmation};

    use crate::event_processor::EventHandler;
    use crate::evm::finalizer::Finalization;
    use crate::evm::json_rpc::MockEthereumClient;
    use crate::handlers::aleo_verify_verifier_set::PollStartedEvent;
    // use crate::handlers::evm_verify_verifier_set::PollStartedEvent;
    use crate::handlers::tests::{into_structured_event, participants};
    use crate::types::TMAddress;
    use crate::PREFIX;

    #[test]
    fn aleo_verify_verifier_set_should_deserialize_correct_event() {
        let config = config(None);

        let event: Event = into_structured_event(
            poll_started_event(&config),
            &TMAddress::random(PREFIX),
        );
        let event: PollStartedEvent = event.try_into().unwrap();
        println!("event: {event:#?}");

        goldie::assert_debug!(event);
    }

    // #[async_test]
    // async fn should_skip_expired_poll() {
    //     let mut rpc_client = MockEthereumClient::new();
    //     // mock the rpc client as erroring. If the handler successfully ignores the poll, we won't hit this
    //     rpc_client.expect_finalized_block().returning(|| {
    //         Err(Report::from(ProviderError::CustomError(
    //             "failed to get finalized block".to_string(),
    //         )))
    //     });
    //
    //     let voting_verifier = TMAddress::random(PREFIX);
    //     let verifier = TMAddress::random(PREFIX);
    //     let expiration = 100u64;
    //     let event: Event = into_structured_event(
    //         poll_started_event(participants(5, Some(verifier.clone())), expiration),
    //         &voting_verifier,
    //     );
    //
    //     let (tx, rx) = watch::channel(expiration - 1);
    //
    //     let handler = super::Handler::new(
    //         verifier,
    //         voting_verifier,
    //         ChainName::from_str("ethereum").unwrap(),
    //         Finalization::RPCFinalizedBlock,
    //         rpc_client,
    //         rx,
    //     );
    //
    //     // poll is not expired yet, should hit rpc error
    //     assert!(handler.handle(&event).await.is_err());
    //
    //     let _ = tx.send(expiration + 1);
    //
    //     // poll is expired, should not hit rpc error now
    //     assert_eq!(handler.handle(&event).await.unwrap(), vec![]);
    // }
    //
    struct Config {
        transition: Transition,
        key_type: KeyType,
        verifier_set: VerifierSet,
        poll_id: PollId,
        source_chain: ChainName,
        source_gateway_address: String,
        confirmation_height: u64,
        expires_at: u64,
        participants: Vec<TMAddress>,
    }

    fn config(verifier: Option<TMAddress>) -> Config {
        let transition =
            Transition::from_str("au17kdp7a7p6xuq6h0z3qrdydn4f6fjaufvzvlgkdd6vzpr87lgcgrq8qx6st")
                .unwrap();
        let key_type = KeyType::AleoSchnorr;
        let verifier_set = build_verifier_set(key_type, &aleo_schnorr_test_data::signers());
        let poll_id = PollId::from_str("100").unwrap();
        let source_chain = ChainName::from_str("aleo-2").unwrap();
        let source_gateway_address = "mygateway".to_string();
        let confirmation_height = 15;
        let expires_at = 100u64;
        let participants = participants(5, verifier);

        Config {
            transition,
            key_type,
            verifier_set,
            poll_id,
            source_chain,
            source_gateway_address,
            confirmation_height,
            expires_at,
            participants,
        }
    }

    fn poll_started_event(config: &Config) -> PollStarted {
        PollStarted::VerifierSet {
            #[allow(deprecated)] // TODO: The below event uses the deprecated tx_id and event_index fields. Remove this attribute when those fields are removed
            verifier_set: VerifierSetConfirmation {
                tx_id: "foo".to_string().parse().unwrap(), // this field is deprecated
                event_index: 0u32, // this field is deprecated
                message_id: config.transition.to_string().parse().unwrap(),
                verifier_set: config.verifier_set.clone(),
            },
            metadata: PollMetadata {
                poll_id: config.poll_id,
                source_chain: config.source_chain.clone(),
                source_gateway_address: config.source_gateway_address.parse().unwrap(),
                confirmation_height: config.confirmation_height,
                expires_at: config.expires_at,
                participants: config.participants.iter()
                    .map(|addr| cosmwasm_std::Addr::unchecked(addr.to_string()))
                    .collect(),

            },
        }
    }
}
