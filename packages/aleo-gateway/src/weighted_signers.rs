use aleo_types::address::Address;
use cosmwasm_std::Uint128;
use error_stack::Report;
use multisig::key::PublicKey;
use multisig::verifier_set::VerifierSet;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::weighted_signer::WeightedSigner;
use crate::{AleoValue, Error};

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WeightedSigners<const GROUP_SIZE: usize = 2, const GROUPS: usize = 2> {
    #[serde_as(as = "[[_; GROUP_SIZE]; GROUPS]")]
    pub signers: [[WeightedSigner; GROUP_SIZE]; GROUPS],
    threshold: Uint128,
    // nonce: [u64; 4], // TODO: this should be included before going to main net
}

impl<const GROUP_SIZE: usize, const GROUPS: usize> TryFrom<&VerifierSet>
    for WeightedSigners<GROUP_SIZE, GROUPS>
{
    type Error = Report<Error>;

    fn try_from(value: &VerifierSet) -> Result<Self, Self::Error> {
        if value.signers.len() > GROUP_SIZE * GROUPS {
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
            .take(GROUP_SIZE * GROUPS)
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
        let _nonce = [0, 0, 0, value.created_at];

        // Distribute signers across groups
        let mut grouped_signers = Vec::with_capacity(GROUPS);
        for group_idx in 0..GROUPS {
            let start = group_idx * GROUP_SIZE;
            let end = start + GROUP_SIZE;

            let group_vec: Vec<_> = signers[start..end].to_vec();
            let group_array: [WeightedSigner; GROUP_SIZE] = group_vec.try_into().expect(&format!(
                "Group {} should have exactly {} elements",
                group_idx, GROUP_SIZE
            ));

            grouped_signers.push(group_array);
        }

        // Convert Vec<[WeightedSigner; GROUP_SIZE]> to [[WeightedSigner; GROUP_SIZE]; GROUPS]
        let signers_array: [[WeightedSigner; GROUP_SIZE]; GROUPS] = grouped_signers
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert to array of size {}", GROUPS));

        Ok(WeightedSigners {
            signers: signers_array,
            threshold,
            // nonce,
        })
    }
}

impl<const GROUP_SIZE: usize, const GROUPS: usize> AleoValue
    for WeightedSigners<GROUP_SIZE, GROUPS>
{
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        // Start with the opening part of the string
        let mut res = String::from("{ signers: [ ");

        // Add each group's formatted string
        for (i, group) in self.signers.iter().enumerate() {
            let group_str = format!(
                "[{}]",
                group
                    .iter()
                    .map(|s| s.to_aleo_string())
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            );

            res.push_str(&group_str);

            // Add comma if not the last group
            if i < self.signers.len() - 1 {
                res.push_str(", ");
            }
        }

        // Add the threshold and closing part
        res.push_str(&format!(" ], quorum: {}u128 }}", self.threshold));

        Ok(res)
    }
    // fn to_aleo_string(&self) -> Result<String, Report<Error>> {
    //     let res = format!(
    //         r#"{{ signers: [ [{}], [{}] ], threshold: {}u128 }}"#,
    //         // r#"{{ signers: [ {}, {} ], threshold: {}u128, nonce: [ {}u64, {}u64, {}u64, {}u64 ] }}"#,
    //         self.signers[0]
    //             .iter()
    //             .map(|s| s.to_aleo_string())
    //             .collect::<Result<Vec<_>, _>>()?
    //             .join(", "),
    //         self.signers[1]
    //             .iter()
    //             .map(|s| s.to_aleo_string())
    //             .collect::<Result<Vec<_>, _>>()?
    //             .join(", "),
    //         self.threshold,
    //         // self.nonce[0],
    //         // self.nonce[1],
    //         // self.nonce[2],
    //         // self.nonce[3]
    //     );
    //
    //     Ok(res)
    // }
}
