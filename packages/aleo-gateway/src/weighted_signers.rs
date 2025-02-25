use aleo_types::address::Address;
use cosmwasm_std::Uint128;
use error_stack::Report;
use multisig::key::PublicKey;
use multisig::verifier_set::VerifierSet;

use crate::weighted_signer::WeightedSigner;
use crate::{AleoValue, Error};

#[derive(Debug, Clone)]
pub struct WeightedSigners {
    signers: Vec<WeightedSigner>, // TODO: [WeightedSigner; 32],
    threshold: Uint128,
    nonce: [u64; 4],
}

impl TryFrom<&VerifierSet> for WeightedSigners {
    type Error = Report<Error>;

    fn try_from(value: &VerifierSet) -> Result<Self, Self::Error> {
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
            .take(32)
            .collect::<Result<Vec<_>, _>>()?;

        signers.sort_by(|signer1, signer2| signer1.signer.cmp(&signer2.signer));

        let threshold = value.threshold;
        let nonce = [0, 0, 0, value.created_at];

        Ok(WeightedSigners {
            signers,
            threshold,
            nonce,
        })
    }
}

impl AleoValue for WeightedSigners {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let signers = self
            .signers
            .iter()
            .map(WeightedSigner::to_aleo_string)
            .collect::<Result<Vec<_>, Report<Error>>>()?
            .join(", ");
        let res = format!(
            r#"{{ signers: [ {} ], threshold: {}u128, nonce: [ {}u64, {}u64, {}u64, {}u64 ] }}"#,
            signers, self.threshold, self.nonce[0], self.nonce[1], self.nonce[2], self.nonce[3]
        );

        Ok(res)
    }
}
