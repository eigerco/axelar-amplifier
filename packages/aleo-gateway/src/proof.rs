use std::ptr::addr_of_mut;

use aleo_types::address::Address;
use error_stack::Report;
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;

use crate::raw_signature::RawSignature;
use crate::weighted_signers::WeightedSigners;
use crate::{AleoValue, Error};

#[derive(Clone, Debug)]
pub struct Proof {
    pub weighted_signer: Address,
    pub signature: RawSignature,
    pub nonce: [u128; 2],
}

impl Proof {
    pub fn new(
        verifier_set: VerifierSet,
        signer_with_signature: SignerWithSig,
    ) -> Result<Self, Report<Error>> {
        let address = verifier_set
            .signers
            .values()
            .next()
            .map(|verifier| match &verifier.pub_key {
                multisig::key::PublicKey::AleoSchnorr(key) => {
                    Ok(Address::try_from(key).map_err(|e| {
                        Report::new(Error::AleoGateway(format!(
                            "Failed to parse address: {}",
                            e
                        )))
                    })?)
                }
                multisig::key::PublicKey::Ecdsa(_) => Err(Report::new(
                    Error::UnsupportedPublicKey("received Ecdsa".to_string()),
                )),
                multisig::key::PublicKey::Ed25519(_) => Err(Report::new(
                    Error::UnsupportedPublicKey("received Ed25519".to_string()),
                )),
            })
            .ok_or_else(|| Report::new(Error::AleoGateway("No signers found".to_string())))??;

        let signature = RawSignature {
            signature: match signer_with_signature.signature {
                multisig::key::Signature::AleoSchnorr(sig) => sig.to_vec(),
                _ => {
                    return Err(Report::new(Error::UnsupportedPublicKey(
                        "Missing Aleo schnorr signature".to_string(),
                    )))
                }
            },
        };

        Ok(Proof {
            weighted_signer: address,
            signature,
            nonce: [3, 1],
        })
    }
}

impl AleoValue for Proof {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ weighted_signer: {}, signaturee: {}, nonce: [ {}u128, {}u128 ] }}"#,
            self.weighted_signer.to_string(),
            self.signature.to_aleo_string()?,
            self.nonce[0],
            self.nonce[1],
        );

        Ok(res)
    }
}
