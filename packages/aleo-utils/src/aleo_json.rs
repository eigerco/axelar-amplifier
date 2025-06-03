use serde::Deserialize;
pub mod deserialize;
pub mod parse;

#[derive(Debug, Clone)]
pub enum AleoJson {
    GasPaidForContractCall(GasPaidForContractCall),
    GasAddedOrRefunded(GasAddedOrRefunded),
    Untargeted(String), // can be parsed but we are not interested in it
    #[allow(dead_code)] // may be useful for debugging
    NotYetParsable(String), // means nom parser currently does not support it
    CallContract(CallContract),
    RemoteDeployInterchainToken(RemoteDeployInterchainToken),
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct GasPaidForContractCall {
    pub gas: u64,
    pub refund_address: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GasAddedOrRefunded {
    pub gas: u64,
    pub tx_hash: String,
    pub log_index: usize,
    pub refund_address: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CallContract {
    pub caller: String,
    pub sender: String,
    pub destination_chain: [u128; 2],
    pub destination_address: [u128; 6],
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeployInterchainToken {
    pub name: u128,
    pub symbol: u128,
    pub decimals: u8,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RemoteDeployInterchainToken {
    pub info: DeployInterchainToken,
    pub token_id: [u128; 2],
    pub destination_chain: [u128; 2],
    pub has_minter: bool,
    pub minter: [u128; 6],
}
