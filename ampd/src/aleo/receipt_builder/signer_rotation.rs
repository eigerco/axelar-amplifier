use aleo_gateway::WeightedSigners;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct SignerRotation {
    pub(crate) message_id: String,
    pub(crate) block_height: u32,
    pub(crate) signers_hash: String,
    pub(crate) weighted_signers: WeightedSigners,
}
