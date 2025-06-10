use aleo_gateway::WeightedSigners;
use axelar_wasm_std::voting::Vote;
use snarkvm::prelude::Network;
use tracing::warn;

use super::{CallContractReceipt, SignerRotation};
use crate::aleo::receipt_builder::Receipt;
use crate::handlers::aleo_verify_msg::Message;
use crate::handlers::aleo_verify_verifier_set::VerifierSetConfirmation;

pub fn verify_message<N: Network>(
    receipt: &Receipt<CallContractReceipt<N>>,
    msg: &Message<N>,
) -> Vote {
    let res = match receipt {
        Receipt::Found(transition_receipt) => transition_receipt == msg,
        Receipt::NotFound(transition, e) => {
            warn!("AleoMessageId: {:?} is not verified: {:?}", transition, e);

            false
        }
    };

    match res {
        true => Vote::SucceededOnChain,
        false => Vote::FailedOnChain,
    }
}

pub fn verify_verifier_set(
    receipt: &Receipt<SignerRotation>,
    msg: &VerifierSetConfirmation,
) -> Vote {
    let res = match receipt {
        Receipt::Found(signer_rotation) => WeightedSigners::try_from(&msg.verifier_set)
            .is_ok_and(|other| other == signer_rotation.weighted_signers),
        Receipt::NotFound(transition, e) => {
            warn!("AleoMessageId: {:?} is not verified: {:?}", transition, e);

            false
        }
    };

    match res {
        true => Vote::SucceededOnChain,
        false => Vote::FailedOnChain,
    }
}
