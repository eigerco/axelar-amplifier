use std::collections::HashMap;
use std::mem::MaybeUninit;

use aleo_types::address::Address;
use cosmwasm_std::HexBinary;
use error_stack::{ensure, Report};
use multisig::key::{PublicKey, Signature};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;

use crate::raw_signature::RawSignature;
use crate::{AleoValue, Array2D, Error, WeightedSigners};

#[derive(Clone, Debug)]
pub struct Proof {
    pub weighted_signers: WeightedSigners,
    pub signature: Array2D<RawSignature>,
    // pub nonce: [u128; 2], // TODO: this should be included before going to mainnet
}

impl Proof {
    pub fn new(
        verifier_set: VerifierSet,
        signer_with_signature: Vec<SignerWithSig>,
    ) -> Result<Self, Report<Error>> {
        let weighted_signers = WeightedSigners::try_from(&verifier_set)?;

        let signer_with_signature_len = signer_with_signature.len();
        let mut address_signature: HashMap<Address, HexBinary> = signer_with_signature
            .into_iter()
            .filter_map(|signer_with_signature| {
                let (key, sig) = match (
                    signer_with_signature.signer.pub_key,
                    signer_with_signature.signature,
                ) {
                    (PublicKey::AleoSchnorr(key), Signature::AleoSchnorr(sig)) => (key, sig),
                    _ => return None,
                };

                let addr = Address::try_from(&key).ok()?;
                Some((addr, sig))
            })
            .collect();

        ensure!(
            address_signature.len() == signer_with_signature_len,
            Error::MismatchedSignerCount {
                address_signatures: address_signature.len(),
                signer_signatures: signer_with_signature_len
            },
        );

        let mut signature: Array2D<MaybeUninit<RawSignature>> =
            unsafe { MaybeUninit::uninit().assume_init() };

        for (group_idx, signer_group) in weighted_signers.signers.iter().enumerate() {
            for (signer_idx, weighted_signer) in signer_group.iter().enumerate() {
                if let Some(sig) = address_signature.remove(&weighted_signer.signer) {
                    signature[group_idx][signer_idx].write(RawSignature {
                        signature: sig.into(),
                    });
                } else {
                    signature[group_idx][signer_idx].write(RawSignature::default());
                }
            }
        }

        let signature = unsafe { std::mem::transmute::<_, Array2D<RawSignature>>(signature) };

        Ok(Proof {
            weighted_signers,
            signature,
            // nonce: [3, 1],
        })
    }
}

impl AleoValue for Proof {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ weighted_signers: {}, signatures: [ {} ] }}"#,
            self.weighted_signers.to_aleo_string()?,
            self.signature
                .iter()
                .map(|sig| {
                    format!(
                        "[{}]",
                        sig.iter()
                            .map(|s| s.to_aleo_string())
                            .collect::<Result<Vec<_>, Report<Error>>>()
                            .unwrap() // for this to fail signature must have non UTF-8 characters
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        );

        Ok(res)
    }
}
