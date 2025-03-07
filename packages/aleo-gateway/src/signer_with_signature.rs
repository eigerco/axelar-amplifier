use crate::raw_signature::RawSignature;
use crate::weighted_signer::WeightedSigner;

#[derive(Clone, Debug)]
pub struct SignerWithSignature {
    pub signer: WeightedSigner,
    pub signature: RawSignature,
}
