use std::str::FromStr;

use aleo_types::address::Address;
use aleo_types::program::Program;
use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use error_stack::{ensure, Report, Result, ResultExt};
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use snarkvm_cosmwasm::program::Network;

use crate::aleo::error::Error;
use crate::aleo::http_client::ClientTrait;
use crate::aleo::utils::*;

mod call_contract;
mod receipt;
mod signer_rotation;

pub use call_contract::{CallContract, CallContractReceipt};
pub use receipt::Receipt;
pub use signer_rotation::SignerRotation;

// State types for the type state pattern
/// Initial state of the builder
#[derive(Debug, Serialize, Deserialize)]
pub struct Initial;

/// State after finding the transaction ID from a transition ID
#[derive(Debug, Serialize, Deserialize)]
pub struct StateTransactionId {
    transaction_id: Transaction,
}

/// State after retrieving the transaction
#[derive(Debug, Deserialize)]
pub struct StateTransactionFound {
    transaction: aleo_utils::block_processor::Transaction,
}

/// State after finding the transition in the transaction
#[derive(Debug, Deserialize)]
pub struct StateTransitionFound {
    transaction: aleo_utils::block_processor::Transaction,
    transition: aleo_utils::block_processor::Transition,
}

/// Builder for verifying Aleo receipts using a type-state pattern
///
/// The builder progresses through multiple states to verify a receipt:
/// 1. Initial → Find transaction ID from transition ID
/// 2. StateTransactionId → Retrieve transaction
/// 3. StateTransactionFound → Find target transition
/// 4. StateTransitionFound → Verify receipt (CallContract or SignerRotation)
pub struct ReceiptBuilder<'a, C: ClientTrait, S> {
    client: &'a C,
    target_contract: Program, // The target program can be the CallContract or the
    state: S,
}

impl<'a, C> ReceiptBuilder<'a, C, Initial>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn new(client: &'a C, target_contract: &'a str) -> Result<Self, Error> {
        let target_contract = Program::from_str(target_contract)
            .change_context(Error::InvalidProgramName(target_contract.to_string()))?;

        Ok(Self {
            client,
            target_contract,
            state: Initial,
        })
    }

    pub async fn get_transaction_id(
        self,
        transition_id: &Transition,
    ) -> Result<ReceiptBuilder<'a, C, StateTransactionId>, Error> {
        let transaction_id = self
            .client
            .find_transaction(transition_id)
            .await
            .change_context(Error::TransitionNotFound(transition_id.to_string()))?;

        let transaction = transaction_id.trim_matches('"');

        Ok(ReceiptBuilder {
            client: self.client,
            target_contract: self.target_contract,
            state: StateTransactionId {
                transaction_id: Transaction::from_str(transaction)
                    .change_context(Error::TransactionNotFound(transaction.to_string()))?,
            },
        })
    }
}

impl<'a, C> ReceiptBuilder<'a, C, StateTransactionId>
where
    C: ClientTrait + Send + Sync + 'static,
{
    /// Retrieve the transaction from the transaction ID and transition to the next state
    pub async fn get_transaction(
        self,
    ) -> Result<ReceiptBuilder<'a, C, StateTransactionFound>, Error> {
        let transaction = self
            .client
            .get_transaction(&self.state.transaction_id)
            .await
            .change_context(Error::TransactionNotFound(
                self.state.transaction_id.to_string(),
            ))?;

        Ok(ReceiptBuilder {
            client: self.client,
            target_contract: self.target_contract,
            state: StateTransactionFound { transaction },
        })
    }
}

impl<'a, C> ReceiptBuilder<'a, C, StateTransactionFound>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn get_transition(self) -> Result<ReceiptBuilder<'a, C, StateTransitionFound>, Error> {
        let transition = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .find(|t| t.program.as_str() == self.target_contract.as_str())
            .ok_or(Error::TransitionNotFoundInTransaction(
                self.target_contract.to_string(),
            ))?
            .clone();

        Ok(ReceiptBuilder {
            client: self.client,
            target_contract: self.target_contract,
            state: StateTransitionFound {
                transaction: self.state.transaction,
                transition,
            },
        })
    }
}

impl<'a, C> ReceiptBuilder<'a, C, StateTransitionFound>
where
    C: ClientTrait + Send + Sync + 'static,
{
    pub fn check_call_contract<N: Network>(self) -> Result<Receipt<CallContractReceipt<N>>, Error> {
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

        Ok(Receipt::Found(CallContractReceipt {
            transition: Transition::from_str(self.state.transition.id.as_str())
                .map_err(|e| Report::new(Error::CalledContractReceipt(e.to_string())))?,
            destination_address: call_contract.destination_address(),
            destination_chain: ChainName::try_from(call_contract.destination_chain())
                .change_context(Error::InvalidChainName)?,
            source_address: Address::from_str(call_contract.sender.to_string().as_ref())
                .change_context(Error::InvalidSourceAddress)?,
            payload: parsed_output.payload,
            n: std::marker::PhantomData,
        }))
    }

    pub fn check_signer_rotation(self) -> Result<Receipt<SignerRotation>, Error> {
        let outputs = self.state.transition.outputs;
        let signer_rotation =
            find_signer_rotation(&outputs).ok_or(Error::SignerRotationNotFound)?;
        let scm = self.state.transition.scm.as_str();

        let signers_rotation_calls = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .filter(|t| {
                t.scm == scm
                    && t.program == self.target_contract.as_str()
                    && t.id != self.state.transition.id
            })
            .count();

        ensure!(signers_rotation_calls == 1, Error::SignerRotationNotFound);

        Ok(Receipt::Found(signer_rotation))
    }
}
