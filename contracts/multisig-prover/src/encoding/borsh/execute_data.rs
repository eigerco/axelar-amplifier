use crate::{error::ContractError, payload::Payload};
use axelar_wasm_std::hash::Hash;
use cosmwasm_std::{HexBinary, Uint256};
use multisig::{
    key::{PublicKey, Recoverable, Signature},
    msg::SignerWithSig,
    worker_set::WorkerSet,
};

#[derive(Debug, thiserror::Error)]
enum ProofError<'a> {
    #[error("Signer not found in the given worker set: {0}")]
    SignerNotInWorkerSet(&'a str),
}

impl<'a> From<ProofError<'a>> for ContractError {
    fn from(error: ProofError<'a>) -> Self {
        use ContractError::*;
        use ProofError::*;
        match error {
            error @ SignerNotInWorkerSet(_) => InvalidPublicKey {
                reason: error.to_string(),
            },
        }
    }
}

struct WeightedSignature<'a> {
    pubkey: &'a PublicKey,
    signature: &'a Signature,
    weight: &'a Uint256,
}

impl<'a> From<&'a SignerWithSig> for WeightedSignature<'a> {
    fn from(value: &'a SignerWithSig) -> Self {
        Self {
            pubkey: &value.signer.pub_key,
            signature: &value.signature,
            weight: &value.signer.weight,
        }
    }
}

struct Proof<'a> {
    signatures: Vec<WeightedSignature<'a>>,
    threshold: &'a Uint256,
    nonce: u64,
}

impl<'a> Proof<'a> {
    fn new(
        worker_set: &'a WorkerSet,
        signers_with_sigs: &'a mut [SignerWithSig],
    ) -> Result<Self, ProofError<'a>> {
        // Signatures are sorted in ascending order
        signers_with_sigs.sort_by(|a, b| a.signer.pub_key.cmp(&b.signer.pub_key));

        let mut signatures: Vec<WeightedSignature> = Vec::with_capacity(signers_with_sigs.len());
        for signer in signers_with_sigs.iter() {
            // Check if the worker set contains this signer.
            if !worker_set
                .signers
                .contains_key(signer.signer.address.as_str())
            {
                return Err(ProofError::SignerNotInWorkerSet(
                    signer.signer.address.as_str(),
                ));
            }
            signatures.push(signer.into())
        }

        Ok(Proof {
            signatures,
            threshold: &worker_set.threshold,
            nonce: worker_set.created_at,
        })
    }
}

pub fn encode(
    worker_set: &WorkerSet,
    mut signers: Vec<SignerWithSig>,
    payload_digest: &Hash,
    _payload: &Payload,
) -> Result<HexBinary, ContractError> {
    for signer in &mut signers {
        recover_signature(payload_digest, signer)?;
    }

    let proof = Proof::new(worker_set, &mut signers);

    todo!()
}

fn recover_signature(message: &[u8], signer: &mut SignerWithSig) -> Result<(), ContractError> {
    let Signature::Ecdsa(non_recoverable) = &signer.signature else {
        return Ok(());
    };

    let signature =
        k256::ecdsa::Signature::from_slice(non_recoverable.as_ref()).map_err(|err| {
            ContractError::InvalidSignature {
                reason: err.to_string(),
            }
        })?;

    let recovery_byte = k256::ecdsa::VerifyingKey::from_sec1_bytes(signer.signer.pub_key.as_ref())
        .and_then(|pubkey| {
            k256::ecdsa::RecoveryId::trial_recovery_from_prehash(&pubkey, message, &signature)
        })
        .map_err(|err| ContractError::InvalidSignature {
            reason: err.to_string(),
        })?;

    let signature_bytes = signature.to_bytes();
    let mut recoverable_bytes = Vec::with_capacity(Recoverable::LEN);
    recoverable_bytes.extend(signature_bytes.iter());
    recoverable_bytes.push(recovery_byte.to_byte());

    let recoverable =
        HexBinary::from(recoverable_bytes)
            .try_into()
            .map_err(
                |err: multisig::ContractError| ContractError::InvalidSignature {
                    reason: err.to_string(),
                },
            )?;
    signer.signature = Signature::EcdsaRecoverable(recoverable);
    Ok(())
}
