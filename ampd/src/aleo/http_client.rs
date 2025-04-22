use aleo_types::transaction::Transaction;
use aleo_types::transition::Transition;
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use mockall::automock;

use crate::aleo::error::Error;
use crate::url::Url;

#[automock]
#[async_trait]
pub trait ClientTrait: Send {
    async fn get_transaction(
        &self,
        transaction_id: &Transaction,
    ) -> Result<aleo_utils::block_processor::Transaction, Error>;

    async fn find_transaction(&self, transition_id: &Transition) -> Result<String, Error>;
}

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: Url,
    network: String,
}

impl Client {
    pub fn new(client: reqwest::Client, base_url: Url, network: String) -> Self {
        Self {
            client,
            base_url,
            network,
        }
    }
}

#[async_trait]
impl ClientTrait for Client {
    #[tracing::instrument(skip(self))]
    async fn get_transaction(
        &self,
        transaction_id: &Transaction,
    ) -> Result<aleo_utils::block_processor::Transaction, Error> {
        const ENDPOINT: &str = "transaction";
        let url = format!(
            "{}{}/{ENDPOINT}/{}",
            self.base_url, self.network, &transaction_id
        );

        tracing::debug!(%url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .change_context(Error::Request)?;

        let transaction: aleo_utils::block_processor::Transaction =
            serde_json::from_str(&response.text().await.change_context(Error::Request)?)
                .change_context(Error::Request)?;

        Ok(transaction)
    }

    #[tracing::instrument(skip(self))]
    async fn find_transaction(&self, transition_id: &Transition) -> Result<String, Error> {
        const ENDPOINT: &str = "find/transactionID";
        let url = format!(
            "{}{}/{ENDPOINT}/{}",
            self.base_url, self.network, &transition_id
        );

        tracing::debug!(%url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .change_context(Error::Request)?;

        response.text().await.change_context(Error::Request)
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use aleo_types::program::Program;
    use snarkvm_cosmwasm::network::TestnetV0;

    use super::*;
    use crate::aleo::ReceiptBuilder;

    pub fn mock_client_1() -> MockClientTrait {
        let mut mock_client = MockClientTrait::new();

        let transaction_id = "at1dgmvx30f79wt6w8fcjurwtsc5zak4efg4ayyme79862xylve7gxsq3nfh6";
        let mut expected_transitions: HashMap<
            Transaction,
            aleo_utils::block_processor::Transaction,
        > = HashMap::new();
        let transaction_one = include_str!(
            "../tests/at1dgmvx30f79wt6w8fcjurwtsc5zak4efg4ayyme79862xylve7gxsq3nfh6.json"
        );
        let snark_tansaction: aleo_utils::block_processor::Transaction =
            serde_json::from_str(transaction_one).unwrap();
        let transaction = Transaction::from_str(transaction_id).unwrap();
        expected_transitions.insert(transaction, snark_tansaction);

        mock_client
            .expect_get_transaction()
            .returning(move |transaction| {
                Ok(expected_transitions.get(transaction).unwrap().clone())
            });

        mock_client.expect_find_transaction().returning(move |_| {
            let transaction_id = "at1dgmvx30f79wt6w8fcjurwtsc5zak4efg4ayyme79862xylve7gxsq3nfh6";
            Ok(transaction_id.to_string())
        });

        mock_client
    }

    pub fn mock_client_2() -> MockClientTrait {
        let mut mock_client = MockClientTrait::new();

        let transaction_id = "at14gry4nauteg5sp00p6d2pj93dhpsm5857ml8y3xg57nkpszhav9qk0tgvd";
        let mut expected_transitions: HashMap<
            Transaction,
            aleo_utils::block_processor::Transaction,
        > = HashMap::new();
        let transaction_one = include_str!(
            "../tests/at14gry4nauteg5sp00p6d2pj93dhpsm5857ml8y3xg57nkpszhav9qk0tgvd.json"
        );
        let snark_tansaction: aleo_utils::block_processor::Transaction =
            serde_json::from_str(transaction_one).unwrap();
        let transaction = Transaction::from_str(transaction_id).unwrap();
        expected_transitions.insert(transaction, snark_tansaction);

        mock_client
            .expect_get_transaction()
            .returning(move |transaction| {
                Ok(expected_transitions.get(transaction).unwrap().clone())
            });

        mock_client.expect_find_transaction().returning(move |_| {
            let transaction_id = "at14gry4nauteg5sp00p6d2pj93dhpsm5857ml8y3xg57nkpszhav9qk0tgvd";
            Ok(transaction_id.to_string())
        });

        mock_client
    }

    #[tokio::test]
    async fn sanity_test1() {
        let client = mock_client_1();
        let transision_id = "au1zn24gzpgkr936qv49g466vfccg8aykcv05rk39s239hjxwrtsu8sltpsd8";
        let transition = Transition::from_str(transision_id).unwrap();
        let gateway_contract = "gateway_base.aleo";
        let program = Program::from_str(gateway_contract).unwrap();

        let res = ReceiptBuilder::new(&client, &program)
            .unwrap()
            .get_transaction_id(&transition)
            .await
            .unwrap()
            .get_transaction()
            .await
            .unwrap()
            .get_transition()
            .unwrap()
            .check_call_contract::<TestnetV0>();
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn sanity_test2() {
        let client = mock_client_2();
        let transision_id = "au17kdp7a7p6xuq6h0z3qrdydn4f6fjaufvzvlgkdd6vzpr87lgcgrq8qx6st";
        let transition = Transition::from_str(transision_id).unwrap();
        let gateway_contract = "ac64caccf8221554ec3f89bf.aleo";
        let program = Program::from_str(gateway_contract).unwrap();

        let res = ReceiptBuilder::new(&client, &program)
            .unwrap()
            .get_transaction_id(&transition)
            .await
            .unwrap()
            .get_transaction()
            .await
            .unwrap()
            .get_transition()
            .unwrap()
            .check_call_contract::<TestnetV0>();
        assert!(res.is_ok());
    }
}
