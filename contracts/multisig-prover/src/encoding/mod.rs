pub mod abi;
pub mod sol;

use cosmwasm_schema::cw_serde;

#[cw_serde]
#[derive(Copy)]
pub enum Encoder {
    Abi,
    Bcs,
    Solana
}
