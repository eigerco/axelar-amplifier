use std::str::FromStr;

use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use error_stack::{ensure, Report, Result, ResultExt};
use router_api::ChainName;
use serde::{Deserialize, Serialize};
use snarkvm::prelude::{Address, Network, ProgramID};

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
    transaction: aleo_utils_temp::block_processor::Transaction,
}

/// State after finding the transition in the transaction
#[derive(Debug, Deserialize)]
pub struct StateTransitionFound {
    transaction: aleo_utils_temp::block_processor::Transaction,
    transition: aleo_utils_temp::block_processor::Transition,
}

/// Builder for verifying Aleo receipts using a type-state pattern
///
/// The builder progresses through multiple states to verify a receipt:
/// 1. Initial → Find transaction ID from transition ID
/// 2. StateTransactionId → Retrieve transaction
/// 3. StateTransactionFound → Find target transition
/// 4. StateTransitionFound → Verify receipt (CallContract or SignerRotation)
pub struct ReceiptBuilder<'a, C: ClientTrait, S, N: Network> {
    client: &'a C,
    target_contract: ProgramID<N>,
    state: S,
}

impl<'a, C, N> ReceiptBuilder<'a, C, Initial, N>
where
    C: ClientTrait + Send + Sync + 'static,
    N: Network,
{
    pub fn new(client: &'a C, target_contract: &'a str) -> Result<Self, Error> {
        let target_contract = ProgramID::from_str(target_contract).map_err(|e| {
            Report::new(Error::InvalidProgramName(target_contract.to_string())).attach_printable(e)
        })?;

        Ok(Self {
            client,
            target_contract,
            state: Initial,
        })
    }

    pub async fn get_transaction_id(
        self,
        transition_id: &Transition,
    ) -> Result<ReceiptBuilder<'a, C, StateTransactionId, N>, Error> {
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

impl<'a, C, N> ReceiptBuilder<'a, C, StateTransactionId, N>
where
    C: ClientTrait + Send + Sync + 'static,
    N: Network,
{
    /// Retrieve the transaction from the transaction ID and transition to the next state
    pub async fn get_transaction(
        self,
    ) -> Result<ReceiptBuilder<'a, C, StateTransactionFound, N>, Error> {
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

impl<'a, C, N> ReceiptBuilder<'a, C, StateTransactionFound, N>
where
    C: ClientTrait + Send + Sync + 'static,
    N: Network,
{
    pub fn get_transition(self) -> Result<ReceiptBuilder<'a, C, StateTransitionFound, N>, Error> {
        let transition = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .find(|t| t.program == self.target_contract.to_string())
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

impl<C, N> ReceiptBuilder<'_, C, StateTransitionFound, N>
where
    C: ClientTrait + Send + Sync + 'static,
    N: Network,
{
    pub fn check_call_contract(self) -> Result<Receipt<CallContractReceipt<N>>, Error> {
        let outputs = self.state.transition.outputs;
        ensure!(outputs.len() == 1, Error::CallContractNotFound);

        // The call contract from call contract call
        let call_contract: CallContract<N> = outputs
            .first()
            .map(read_call_contract)
            .ok_or(Error::CallContractNotFound)??;

        let payload = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .find_map(|t| {
                if t.id != self.state.transition.id && t.program != self.target_contract.to_string()
                {
                    find_call_contract_in_outputs::<N>(&t.outputs, call_contract.payload_hash)
                } else {
                    None
                }
            })
            .ok_or(Error::CallContractNotFound)?;

        Ok(Receipt::Found(CallContractReceipt {
            transition: Transition::from_str(self.state.transition.id.as_str())
                .map_err(|e| Report::new(Error::CalledContractReceipt(e.to_string())))?,
            destination_address: call_contract.destination_address(),
            destination_chain: ChainName::try_from(call_contract.destination_chain())
                .change_context(Error::InvalidChainName)?,
            source_address: Address::<N>::from_str(call_contract.sender.to_string().as_ref())
                .map_err(|e| Report::new(Error::InvalidSourceAddress).attach_printable(e))?,
            payload: payload.as_bytes().to_vec(),
            n: std::marker::PhantomData,
        }))
    }

    pub fn check_signer_rotation(self) -> Result<Receipt<SignerRotation>, Error> {
        let outputs = self.state.transition.outputs;
        let signer_rotation = find_in_outputs(&outputs).ok_or(Error::SignerRotationNotFound)?;
        let scm = self.state.transition.scm.as_str();

        let signers_rotation_calls = self
            .state
            .transaction
            .execution
            .transitions
            .iter()
            .filter(|t| {
                t.scm == scm
                    && t.program == self.target_contract.to_string()
                    && t.id != self.state.transition.id
            })
            .count();

        ensure!(signers_rotation_calls == 1, Error::SignerRotationNotFound);

        Ok(Receipt::Found(signer_rotation))
    }
}
