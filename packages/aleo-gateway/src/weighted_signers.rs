use std::mem::MaybeUninit;

use aleo_types::address::{Address, ZERO_ADDRESS};
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
    // nonce: [u64; 4], // TODO: this should be included before going to mainnet
}

impl<const GROUP_SIZE: usize, const GROUPS: usize> TryFrom<&VerifierSet>
    for WeightedSigners<GROUP_SIZE, GROUPS>
{
    type Error = Report<Error>;

    fn try_from(value: &VerifierSet) -> Result<Self, Self::Error> {
        if value.signers.len() > GROUP_SIZE.saturating_mul(GROUPS) {
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
            .take(GROUP_SIZE.saturating_mul(GROUPS))
            .collect::<Result<Vec<_>, _>>()?;

        signers.sort_by(|signer1, signer2| {
            /* give the lowest priority to the default address */
            if signer1.signer == *ZERO_ADDRESS {
                std::cmp::Ordering::Greater
            } else if signer2.signer == *ZERO_ADDRESS {
                std::cmp::Ordering::Less
            } else {
                signer1.signer.cmp(&signer2.signer)
            }
        });

        let threshold = value.threshold;
        let _nonce = [0, 0, 0, value.created_at];

        let mut signature: [[MaybeUninit<WeightedSigner>; GROUP_SIZE]; GROUPS] =
            unsafe { MaybeUninit::uninit().assume_init() };

        for (group_idx, signer_group) in signers.chunks(GROUP_SIZE).enumerate() {
            for (signer_idx, weighted_signer) in signer_group.iter().enumerate() {
                signature[group_idx][signer_idx].write(weighted_signer.clone());
            }
        }

        let signers_array = unsafe {
            std::ptr::read(&signature as *const _ as *const [[WeightedSigner; GROUP_SIZE]; GROUPS])
        };

        Ok(WeightedSigners {
            signers: signers_array,
            threshold,
        })
    }
}

impl<const GROUP_SIZE: usize, const GROUPS: usize> AleoValue
    for WeightedSigners<GROUP_SIZE, GROUPS>
{
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        // Add each group's formatted string
        let signers = self
            .signers
            .iter()
            .map(|group| {
                let group_str = group
                    .iter()
                    // Weighted Signer to_aleo_string does  not produce an error
                    .map(|s| s.to_aleo_string().unwrap())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", group_str)
            })
            .collect::<Vec<_>>()
            .join(", ");

        Ok(format!(
            "{{ signers: [ {} ], quorum: {}u128 }}",
            signers, self.threshold
        ))
    }
}
