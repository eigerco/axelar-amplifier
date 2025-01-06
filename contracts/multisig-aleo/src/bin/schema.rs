use cosmwasm_schema::write_api;
use multisig_aleo::msg::{InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
    }
}
