use std::fmt;
use std::hash::{Hash as StdHash, Hasher};

use cosmrs::AccountId;
use ethers_core::types::{Address, H256};
use serde::{Deserialize, Serialize};

mod key;
pub(crate) mod starknet;
pub use key::{CosmosPublicKey, PublicKey};

pub type EVMAddress = Address;
pub type Hash = H256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TMAddress(AccountId);

impl StdHash for TMAddress {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bytes().hash(state);
    }
}

impl From<AccountId> for TMAddress {
    fn from(account_id: AccountId) -> Self {
        Self(account_id)
    }
}

impl AsRef<AccountId> for TMAddress {
    fn as_ref(&self) -> &AccountId {
        &self.0
    }
}

impl fmt::Display for TMAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
pub mod test_utils {
    use rand::rngs::OsRng;

    use super::CosmosPublicKey;
    use crate::types::TMAddress;

    impl TMAddress {
        pub fn random(prefix: &str) -> Self {
            Self(
                CosmosPublicKey::from(k256::ecdsa::SigningKey::random(&mut OsRng).verifying_key())
                    .account_id(prefix)
                    .expect("failed to convert to account identifier"),
            )
        }
    }
}
