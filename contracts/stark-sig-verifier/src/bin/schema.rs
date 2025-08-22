use cosmwasm_schema::write_api;

use signature_verifier_api::msg::ExecuteMsg;
use stark_sig_verifier::contract::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
    }
}
