//! Verification implementation of Starknet JSON RPC client's verification of
//! transaction existence

use serde::{Deserialize, Serialize};
use starknet_core::types::BlockId;
use starknet_providers::jsonrpc::HttpTransport;
use starknet_providers::{JsonRpcClient, Provider};
use url::Url;

use super::error::Error;

/// Implementor of verification method(s) for given network using JSON RPC
/// client.
pub struct Verifier {
    client: JsonRpcClient<HttpTransport>,
}

impl Verifier {
    /// Constructor.
    /// Expects URL of any JSON RPC entry point of Starknet.
    /// ## Example
    /// ```rust
    ///    use starknet_verifier::{Verifier, STARKNET_GOERLI};
    ///    assert!(Verifier::new(STARKNET_GOERLI).is_ok());
    /// ```
    pub fn new(starknet_url: impl AsRef<str>) -> Result<Self, Error> {
        Ok(Verifier {
            client: JsonRpcClient::new(HttpTransport::new(Url::parse(starknet_url.as_ref())?)),
        })
    }
    /// Using given JSON serialized [EventId] tries to fetch it from given
    /// `starknet_url`. Returns error if request fails, `false` if internal
    /// error returned by querry and `true` if transaction found
    /// ## Example
    /// ```rust
    ///    use starknet_verifier::{Verifier, STARKNET_GOERLI};
    ///    let verifier = Verifier::new(STARKNET_GOERLI).expect("bad url");
    ///    assert!(futures::executor::block_on(verifier.get_event("some bad data...")).is_err());
    /// ```
    pub async fn get_event(&self, event_id: impl AsRef<str>) -> Result<bool, Error> {
        let e_id: EventId = serde_json::from_str(event_id.as_ref())?;
        Ok(self
            .client
            .get_transaction_by_block_id_and_index(e_id.block_id, e_id.transaction_index)
            .await
            .is_ok())
    }
}

/// Borderline identifier of exact transaction in exact block on Starknet
/// network Should be constructed by de-serialization from `String` only, as
/// that's how it's represented within Axelar `Message` Implements reverse
/// conversion by `.to_string()` method.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventId {
    block_id: BlockId,
    transaction_index: u64,
}

impl ToString for EventId {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}
