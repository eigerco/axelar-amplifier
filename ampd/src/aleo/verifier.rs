use aleo_gateway::WeightedSigners;
use axelar_wasm_std::voting::Vote;
use tracing::warn;

use crate::aleo::receipt_builder::{FoundReceipt, Receipt};
use crate::handlers::aleo_verify_msg::Message;
use crate::handlers::aleo_verify_verifier_set::VerifierSetConfirmation;

fn verify(receipt: &Receipt, msg: &Message) -> Vote {
    let res = match receipt {
        Receipt::Found(FoundReceipt::CallContract(transition_receipt)) => transition_receipt == msg,
        Receipt::NotFound(transition, e) => {
            warn!("AleoMessageId: {:?} is not verified: {:?}", transition, e);

            false
        }
        Receipt::Found(FoundReceipt::SignerRotation(_)) => todo!(),
    };

    match res {
        true => Vote::SucceededOnChain,
        false => Vote::FailedOnChain,
    }
}

pub fn verify_message(receipt: &Receipt, msg: &Message) -> Vote {
    verify(receipt, msg)
}

// TODO: use the full message for comparison
pub fn verify_verifier_set(receipt: &Receipt, msg: &VerifierSetConfirmation) -> Vote {
    let res = match receipt {
        Receipt::Found(FoundReceipt::SignerRotation(transition_receipt)) => {
            WeightedSigners::try_from(&msg.verifier_set)
                .map_or(false, |other| other == transition_receipt.weighted_signers)
        }
        Receipt::NotFound(transition, e) => {
            warn!("AleoMessageId: {:?} is not verified: {:?}", transition, e);

            false
        }
        Receipt::Found(FoundReceipt::CallContract(_)) => todo!(),
    };

    match res {
        true => Vote::SucceededOnChain,
        false => Vote::FailedOnChain,
    }
}
