use aleo_types::address::Address;
use cosmwasm_std::Uint128;
use error_stack::Report;
use multisig::key::PublicKey;
use multisig::verifier_set::VerifierSet;

use crate::weighted_signer::WeightedSigner;
use crate::{AleoValue, Error};

#[derive(Debug, Clone)]
pub struct WeightedSigners {
    pub signers: [[WeightedSigner; 32]; 2],
    threshold: Uint128,
    // nonce: [u64; 4], // TODO: this should be included before going to main net
}

impl TryFrom<&VerifierSet> for WeightedSigners {
    type Error = Report<Error>;

    fn try_from(value: &VerifierSet) -> Result<Self, Self::Error> {
        if value.signers.len() > 64 {
            return Err(Report::new(Error::AleoGateway(
                "Too many signers in the verifier set".to_string(),
            )));
        }

        let mut signers = value
            .signers
            .values()
            .map(|signer| match &signer.pub_key {
                PublicKey::AleoSchnorr(key) => Ok(WeightedSigner {
                    signer: Address::try_from(key).map_err(|e| {
                        Report::new(Error::AleoGateway(format!(
                            "Failed to parse address: {}",
                            e
                        )))
                    })?,
                    weight: signer.weight.into(),
                }),
                PublicKey::Ecdsa(_) => Err(Report::new(Error::UnsupportedPublicKey(
                    "received Ecdsa".to_string(),
                ))),
                PublicKey::Ed25519(_) => Err(Report::new(Error::UnsupportedPublicKey(
                    "received Ed25519".to_string(),
                ))),
            })
            .chain(std::iter::repeat_with(|| {
                Ok(WeightedSigner {
                    signer: Address::default(),
                    weight: Default::default(),
                })
            }))
            .take(64)
            .collect::<Result<Vec<_>, _>>()?;

        // signers.sort_by(|signer1, signer2| signer1.signer.cmp(&signer2.signer));
        signers.sort_by(|signer1, signer2| {
            /* give the lowest priority to the default address */
            if signer1.signer == Address::default() {
                std::cmp::Ordering::Greater
            } else if signer2.signer == Address::default() {
                std::cmp::Ordering::Less
            } else {
                signer1.signer.cmp(&signer2.signer)
            }
        });

        let threshold = value.threshold;
        let nonce = [0, 0, 0, value.created_at];

        // TODO: refactor this to be more efficient
        let mut iter = signers.into_iter();
        let first_vec: Vec<_> = iter.by_ref().take(32).collect();
        let second_vec: Vec<_> = iter.collect();

        // Convert to arrays
        let first_array: [WeightedSigner; 32] =
            first_vec.try_into().expect("Should be exactly 32 elements");
        let second_array: [WeightedSigner; 32] = second_vec
            .try_into()
            .expect("Should be exactly 32 elements");

        Ok(WeightedSigners {
            signers: [first_array, second_array],
            threshold,
            // nonce,
        })
    }
}

impl AleoValue for WeightedSigners {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ signers: [ {}, {} ], threshold: {}u128 }}"#,
            // r#"{{ signers: [ {}, {} ], threshold: {}u128, nonce: [ {}u64, {}u64, {}u64, {}u64 ] }}"#,
            self.signers[0]
                .iter()
                .map(|s| s.to_aleo_string())
                .collect::<Result<Vec<_>, _>>()?
                .join(", "),
            self.signers[1]
                .iter()
                .map(|s| s.to_aleo_string())
                .collect::<Result<Vec<_>, _>>()?
                .join(", "),
            self.threshold,
            // self.nonce[0],
            // self.nonce[1],
            // self.nonce[2],
            // self.nonce[3]
        );

        Ok(res)
    }
}
