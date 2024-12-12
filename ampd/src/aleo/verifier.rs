use axelar_wasm_std::voting::Vote;
use tracing::warn;

use super::http_client::Receipt;
use crate::handlers::aleo_verify_msg::Message;

fn verify(
    // _gateway_address: &Program, // TODO: use this
    receipt: &Receipt,
    msg: &Message,
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

pub fn verify_message(
    // _gateway_address: &Program,
    receipt: &Receipt,
    msg: &Message,
) -> Vote {
    verify(receipt, msg)
}
