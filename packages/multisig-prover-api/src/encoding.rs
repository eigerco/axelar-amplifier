use cosmwasm_schema::cw_serde;

#[cw_serde]
#[derive(Copy)]
pub enum Encoder {
    Abi,
    Bcs,
    StellarXdr,
    // TODO: This should be changed to Starknet,
    // but then the multisig-prover must be migrated with a new config value
    StarknetAbi,
}
