use std::collections::HashMap;
use std::mem::MaybeUninit;

use aleo_types::address::Address;
use cosmwasm_std::HexBinary;
use error_stack::{ensure, Report};
use multisig::key::{PublicKey, Signature};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;

use crate::raw_signature::RawSignature;
use crate::{AleoValue, Error, WeightedSigners};

#[derive(Clone, Debug)]
pub struct Proof<const GROUP_SIZE: usize = 2, const GROUPS: usize = 2> {
    pub weighted_signers: WeightedSigners<GROUP_SIZE, GROUPS>,
    pub signature: [[RawSignature; GROUP_SIZE]; GROUPS],
    // pub nonce: [u128; 2], // TODO: this should be included before going to mainnet
}

impl<const GROUP_SIZE: usize, const GROUPS: usize> Proof<GROUP_SIZE, GROUPS> {
    pub fn new(
        verifier_set: VerifierSet,
        signer_with_signature: Vec<SignerWithSig>,
    ) -> Result<Self, Report<Error>> {
        let weighted_signers = WeightedSigners::try_from(&verifier_set)?;

        let signer_with_signature_len = signer_with_signature.len();
        let address_signature: HashMap<Address, HexBinary> = signer_with_signature
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

        let mut signature: [[MaybeUninit<RawSignature>; GROUP_SIZE]; GROUPS] =
            unsafe { MaybeUninit::uninit().assume_init() };

        for (group_idx, signer_group) in weighted_signers.signers.iter().enumerate() {
            for (signer_idx, weighted_signer) in signer_group.iter().enumerate() {
                if let Some(sig) = address_signature.get(&weighted_signer.signer) {
                    signature[group_idx][signer_idx].write(RawSignature {
                        signature: sig.as_slice().to_vec(),
                    });
                } else {
                    signature[group_idx][signer_idx].write(RawSignature { signature: vec![] });
                }
            }
        }

        let signature = unsafe {
            std::ptr::read(&signature as *const _ as *const [[RawSignature; GROUP_SIZE]; GROUPS])
        };

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
        );

        Ok(res)
    }
}
