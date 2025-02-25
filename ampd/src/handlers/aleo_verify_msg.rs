use std::collections::{HashMap, HashSet};

use aleo_types::address::Address as AleoAddress;
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
use tracing::{debug, info, info_span};
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
    pub tx_id: Transition,
    pub destination_address: String,
    pub destination_chain: ChainName,
    pub source_address: AleoAddress,
    pub payload_hash: Hash,
}

#[derive(Deserialize, Debug)]
#[try_from("wasm-messages_poll_started")]
struct PollStartedEvent {
    poll_id: PollId,
    source_chain: ChainName,
    expires_at: u64,
    participants: Vec<TMAddress>,
    messages: Vec<Message>,
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
            messages,
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
        let transitions: HashSet<Transition> = messages.iter().map(|m| m.tx_id.clone()).collect();

        let http_client = AleoClientWrapper::new(&self.http_client);
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
                .map(|msg| msg.tx_id.as_ref())
                .collect::<Vec<&str>>()
                .as_value(),
        )
        .in_scope(|| {
            info!("ready to verify messages in poll");

            let votes: Vec<_> = messages
                .iter()
                .map(|msg| {
                    transition_receipts
                        .get(&msg.tx_id)
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

    use aleo_types::program::Program;
    use cosmrs::AccountId;

    use super::*;
    use crate::types::TMAddress;

    fn poll_started_event() -> Event {
        let expires_at: u64 = 10;
        let participants: Vec<TMAddress> = vec![AccountId::from_str(
            "axelar1a9d3a3hcykzfa8rn3y7d47ns55x3wdlykchydd8x3f95dtz9qh0q3vnrg0",
        )
        .unwrap()
        .into()];
        let messages: Vec<Message> = vec![Message {
            tx_id: Transition::from_str(
                "au17kdp7a7p6xuq6h0z3qrdydn4f6fjaufvzvlgkdd6vzpr87lgcgrq8qx6st",
            )
            .unwrap(),
            destination_address: "dapp20250123exec.aleo".to_string(),
            destination_chain: ChainName::from_str("aleo-2").unwrap(),
            source_address: AleoAddress::from_str(
                "aleo1ejcm4cpwcjtenxlg37utj8dn7s88xs5asvl5u25w4udkd7e7dcpqvaeyaz",
            )
            .unwrap(),
            payload_hash: Hash::from_str(
                "d1af832424763b0b7692605ebfbdd461cc5f4d560228e06b4b6e6191c9e6fb08",
            )
            .unwrap(),
        }];

        let v: Vec<(String, serde_json::Value)> = vec![
            (
                "poll_id".to_string(),
                serde_json::to_value(PollId::from(100)).unwrap(),
            ),
            (
                "_contract_address".to_string(),
                serde_json::to_value(
                    "axelar1a9d3a3hcykzfa8rn3y7d47ns55x3wdlykchydd8x3f95dtz9qh0q3vnrg0",
                )
                .unwrap(),
            ),
            (
                "source_chain".to_string(),
                serde_json::to_value("aleo-2").unwrap(),
            ),
            (
                "source_gateway_address".to_string(),
                serde_json::to_value(Program::from_str("vzevxifdoj.aleo").unwrap()).unwrap(),
            ),
            (
                "expires_at".to_string(),
                serde_json::to_value(expires_at).unwrap(),
            ),
            (
                "participants".to_string(),
                serde_json::to_value(participants).unwrap(),
            ),
            (
                "messages".to_string(),
                serde_json::to_value(messages).unwrap(),
            ),
        ];

        let json_map: serde_json::Map<String, serde_json::Value> = v.into_iter().collect();

        Event::Abci {
            event_type: "wasm-messages_poll_started".to_string(),
            attributes: json_map,
        }
    }

    use cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContract;
    use prost::Message as _;

    #[tokio::test]
    async fn aleo_verify_msg() {
        let mock_client = crate::aleo::http_client::tests::mock_client_3();
        let event = poll_started_event();

        let handler = Handler::new(
            TMAddress::from(
                AccountId::from_str(
                    "axelar1a9d3a3hcykzfa8rn3y7d47ns55x3wdlykchydd8x3f95dtz9qh0q3vnrg0",
                )
                .unwrap(),
            ),
            TMAddress::from(
                AccountId::from_str(
                    "axelar1a9d3a3hcykzfa8rn3y7d47ns55x3wdlykchydd8x3f95dtz9qh0q3vnrg0",
                )
                .unwrap(),
            ),
            ChainName::from_str("aleo-2").unwrap(),
            mock_client,
            tokio::sync::watch::channel(0).1,
            "ac64caccf8221554ec3f89bf.aleo".to_string(),
        );

        let res = handler.handle(&event).await.unwrap();
        let res: Vec<MsgExecuteContract> = res
            .iter()
            .map(|msg| MsgExecuteContract::decode(msg.value.as_slice()).unwrap())
            .collect();

        for r in res {
            let _decode: ExecuteMsg = serde_json::from_slice(&r.msg).unwrap();
        }
    }
}
