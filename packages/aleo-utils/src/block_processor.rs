use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Block {
    pub transactions: Vec<TransactionWrapper>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Accepted,
    Rejected,
}

#[derive(Deserialize, Debug)]
pub struct TransactionWrapper {
    pub status: Status,
    pub transaction: Transaction,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    pub execution: Execution,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Execution {
    pub transitions: Vec<Transition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transition {
    pub id: String,
    pub program: String,
    pub function: String,
    pub outputs: Vec<IdValuePair>,
    pub scm: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IdValuePair {
    pub id: String,
    pub value: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GatewayOutput {
    #[allow(dead_code)]
    pub caller: String,
    pub sender: String,
    pub destination_address: [u8; 20],
    pub destination_chain: [u8; 32],
}

#[cfg(test)]
mod tests {
    use crate::block_processor::{Block, Transaction};

    #[test]
    fn parse_block() {
        let block = include_str!(
            "../../../test/data/aleo-utils/ab1yf27fcu4exccn4jt6wmwuxkseafyxhs5vk92su4nutq35s4jdupqeeprdp.json"
        );

        let block = serde_json::from_str::<Block>(block);
        assert!(block.is_ok());
    }

    #[test]
    fn parse_transaction() {
        let transaction = include_str!(
            "../../../test/data/aleo-utils/at1dgmvx30f79wt6w8fcjurwtsc5zak4efg4ayyme79862xylve7gxsq3nfh6.json"
        );

        let transaction = serde_json::from_str::<Transaction>(transaction);
        assert!(transaction.is_ok());
    }
}
