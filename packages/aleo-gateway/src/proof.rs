use std::collections::HashMap;

use aleo_types::address::Address;
use cosmwasm_std::HexBinary;
use error_stack::Report;
use multisig::key::{PublicKey, Signature};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;

use crate::raw_signature::RawSignature;
use crate::{AleoValue, Error, WeightedSigners};

#[derive(Clone, Debug)]
pub struct Proof<const GROUP_SIZE: usize = 2, const GROUPS: usize = 2> {
    pub weighted_signers: WeightedSigners<GROUP_SIZE, GROUPS>,
    pub signature: [[RawSignature; GROUP_SIZE]; GROUPS],
    // pub nonce: [u128; 2],
}

impl<const GROUP_SIZE: usize, const GROUPS: usize> Proof<GROUP_SIZE, GROUPS> {
    pub fn new(
        verifier_set: VerifierSet,
        signer_with_signature: Vec<SignerWithSig>,
    ) -> Result<Self, Report<Error>> {
        let weighted_signers = WeightedSigners::try_from(&verifier_set)?;

        let mut signer_with_signature = signer_with_signature;
        signer_with_signature.sort_by(|signer1, signer2| {
            let PublicKey::AleoSchnorr(pub_key1) = signer1.signer.pub_key.clone() else {
                todo!();
            };

            let PublicKey::AleoSchnorr(pub_key2) = signer2.signer.pub_key.clone() else {
                todo!();
            };

            let pub_key1 = Address::try_from(&pub_key1).unwrap();
            let pub_key2 = Address::try_from(&pub_key2).unwrap();

            /* give the lowest priority to the default address */
            if pub_key1 == Address::default() {
                std::cmp::Ordering::Greater
            } else if pub_key2 == Address::default() {
                std::cmp::Ordering::Less
            } else {
                pub_key1.cmp(&pub_key2)
            }
        });

        let my_map: HashMap<Address, HexBinary> = signer_with_signature
            .iter()
            .filter_map(|signer_with_signature| {
                // TODO: refactor this to be more efficient
                let PublicKey::AleoSchnorr(key) = signer_with_signature.signer.pub_key.clone()
                else {
                    return None;
                };

                let Signature::AleoSchnorr(sig) = signer_with_signature.signature.clone() else {
                    return None;
                };

                let addr = Address::try_from(&key).ok()?;
                Some((addr, sig))
            })
            .collect();

        // TODO: refactor this to be more efficient
        let mut signature: [[RawSignature; GROUP_SIZE]; GROUPS] =
            core::array::from_fn(|_| core::array::from_fn(|_| RawSignature::default()));

        for (group_idx, signer_group) in weighted_signers.signers.iter().enumerate() {
            for (signer_idx, weighted_signer) in signer_group.iter().enumerate() {
                if weighted_signer.signer == Address::default() {
                    // TODO: break outer
                    break;
                }

                if let Some(sig) = my_map.get(&weighted_signer.signer) {
                    signature[group_idx][signer_idx] = RawSignature {
                        signature: sig.as_slice().to_vec(),
                    };
                }
            }
        }

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
            r#"{{ weighted_signer: {}, signatures: [ {} ] }}"#,
            // r#"{{ weighted_signer: {}, signaturee: [ {} ], nonce: [ {}u128, {}u128 ] }}"#,
            self.weighted_signers.to_aleo_string()?,
            self.signature
                .iter()
                .map(|sig| {
                    format!(
                        "[{}]",
                        sig.iter()
                            .map(|s| s.to_aleo_string())
                            .collect::<Result<Vec<_>, Report<Error>>>()
                            .unwrap() // TODO: remove unwrap
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
            // self.nonce[0],
            // self.nonce[1],
        );

        Ok(res)
    }
}
