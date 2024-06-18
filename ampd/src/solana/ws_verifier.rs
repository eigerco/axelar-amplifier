use axelar_message_primitives::Address;
use axelar_wasm_std::voting::Vote;
use hex::ToHex;

use crate::handlers::solana_verify_worker_set::VerifierSetConfirmation;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use thiserror::Error;
use tracing::{error};

use gmp_gateway::events::GatewayEvent;

#[derive(Error, Debug, PartialEq)]
pub enum VerificationError {
    #[error("Failed to parse tx log messages")]
    NoLogMessages,
    #[error("Tried to get gateway event from program logs, but couldn't find anything.")]
    NoGatewayEventFound,
    #[error("Parsing error: {0}")]
    ParsingError(String),
}

type Result<T> = std::result::Result<T, VerificationError>;

pub fn parse_gateway_event(tx: &EncodedConfirmedTransactionWithStatusMeta) -> Result<GatewayEvent> {
    let Some(meta) = &tx.transaction.meta else {
        return Err(VerificationError::NoLogMessages);
    };

    let log_messages = match &meta.log_messages {
        solana_transaction_status::option_serializer::OptionSerializer::Some(log_msg) => log_msg,
        _ => return Err(VerificationError::NoLogMessages),
    };

    log_messages
        .iter()
        .find_map(GatewayEvent::parse_log)
        .ok_or(VerificationError::NoGatewayEventFound)
}

pub fn verify_verifier_set(
    verifier_set_conf: &VerifierSetConfirmation,
    signers: &Vec<Address>,
    weights: &Vec<u128>,
    quorum: u128,
) -> Vote {
    let verifier_set = &verifier_set_conf.verifier_set;

    if verifier_set.threshold.u128() != quorum {
        return Vote::FailedOnChain;
    }

    if signers.len() != weights.len() {
        return Vote::FailedOnChain;
    }

    for (sol_addr, sol_weight) in signers.iter().zip(weights.iter()) {
        let sol_addr = sol_addr.encode_hex::<String>();
        let Some((addr, signer)) = verifier_set.signers.get_key_value(&sol_addr) else {
            return Vote::FailedOnChain;
        };

        if *addr != signer.address.to_string() {
            return Vote::FailedOnChain;
        }

        if sol_addr != *addr {
            return Vote::FailedOnChain;
        }

        if *sol_weight != signer.weight.u128() {
            return Vote::FailedOnChain;
        }
    }
    Vote::SucceededOnChain
}

#[cfg(test)]
mod tests {
    // todo tests
}
