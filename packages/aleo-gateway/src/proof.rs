use error_stack::Report;
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;

use crate::raw_signature::RawSignature;
use crate::weighted_signers::WeightedSigners;
use crate::{AleoValue, Error};

#[derive(Clone, Debug)]
pub struct Proof {
    pub weighted_signers: WeightedSigners,
    pub signatures: Vec<RawSignature>,
}

impl Proof {
    pub fn new(
        verifier_set: VerifierSet,
        signer_with_signature: Vec<SignerWithSig>,
    ) -> Result<Self, Report<Error>> {
        let weighted_signers = WeightedSigners::try_from(&verifier_set)?;

        let mut signer_with_signature = signer_with_signature;

        signer_with_signature.sort_by(|s1, s2| s1.signer.address.cmp(&s2.signer.address));

        let signatures = signer_with_signature
            .iter()
            .cloned()
            .map(|s| {
                Ok(RawSignature {
                    signature: match s.signature {
                        multisig::key::Signature::AleoSchnorr(sig) => sig.to_vec(),
                        _ => {
                            return Err(Report::new(Error::UnsupportedPublicKey(
                                "Missing Aleo schnorr signature".to_string(),
                            )))
                        }
                    },
                })
            })
            .collect::<Result<Vec<_>, Report<Error>>>()?;

        Ok(Proof {
            weighted_signers,
            signatures,
        })
    }
}

impl AleoValue for Proof {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ weighted_signers: [{}], signatures: [{}] }}"#,
            self.weighted_signers.to_aleo_string()?,
            self.signatures
                .iter()
                .map(|signature| { format!(r#"{}"#, signature.to_aleo_string().unwrap(),) })
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(res)
    }
}
