use std::array::TryFromSliceError;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

use crate::{error::ContractError, payload::Payload};
use axelar_wasm_std::hash::Hash;
use cosmwasm_std::StdError;
use cosmwasm_std::{HexBinary, Uint256};

use multisig::msg::Signer;
use multisig::{
    key::{PublicKey, Recoverable, Signature},
    msg::SignerWithSig,
    worker_set::WorkerSet,
};
use router_api::{CrossChainId, Message};

const ED25519_PUBKEY_LEN: usize = 32;
const ECDSA_COMPRESSED_PUBKEY_LEN: usize = 33;

const ED25519_SIGNATURE_LEN: usize = 64;
const ECDSA_RECOVERABLE_SIGNATURE_LEN: usize = 65;

#[derive(Debug, thiserror::Error)]
enum ProofError {
    #[error("Signer not found in the given worker set: {0}")]
    SignerNotInWorkerSet(String),
    #[error(transparent)]
    StdError(#[from] StdError),
    #[error(transparent)]
    TryFromSliceError(#[from] TryFromSliceError),
    #[error("Can't serialize a non-recoverable ECDSA signature")]
    NonRecoverableSignature,
}

impl From<ProofError> for ContractError {
    fn from(error: ProofError) -> Self {
        use ContractError::*;
        use ProofError::*;
        match error {
            error @ SignerNotInWorkerSet(_) => InvalidPublicKey {
                reason: error.to_string(),
            },
            StdError(_) => todo!(),
            TryFromSliceError(_) => todo!(),
            NonRecoverableSignature => todo!(),
        }
    }
}

type EcdsaPubkey = [u8; ECDSA_COMPRESSED_PUBKEY_LEN];
type Ed25519Pubkey = [u8; ED25519_PUBKEY_LEN];

#[derive(Clone)]
enum OurPublicKey {
    Ecdsa(EcdsaPubkey),
    Ed25519(Ed25519Pubkey),
}

impl TryFrom<PublicKey> for OurPublicKey {
    type Error = StdError;

    fn try_from(value: PublicKey) -> Result<Self, Self::Error> {
        match value {
            PublicKey::Ecdsa(bytes) => bytes
                .to_array::<ECDSA_COMPRESSED_PUBKEY_LEN>()
                .map(OurPublicKey::Ecdsa),
            PublicKey::Ed25519(bytes) => bytes
                .to_array::<ED25519_PUBKEY_LEN>()
                .map(OurPublicKey::Ed25519),
        }
    }
}

type EcdsaRecoverableSignature = [u8; ECDSA_RECOVERABLE_SIGNATURE_LEN];
type Ed25519Signature = [u8; ED25519_SIGNATURE_LEN];

#[derive(Clone)]
enum OurSignature {
    EcdsaRecoverable(EcdsaRecoverableSignature),
    Ed25519(Ed25519Signature),
}

impl TryFrom<Signature> for OurSignature {
    type Error = ProofError;

    fn try_from(value: Signature) -> Result<Self, Self::Error> {
        Ok(match value {
            Signature::Ecdsa(bytes) => Err(ProofError::NonRecoverableSignature)?,
            Signature::EcdsaRecoverable(recoverable) => {
                recoverable.as_ref().try_into().map(OurSignature::Ed25519)?
            }
            Signature::Ed25519(bytes) => bytes
                .to_array::<ED25519_SIGNATURE_LEN>()
                .map(OurSignature::Ed25519)?,
        })
    }
}

struct OurU256([u8; 32]);

impl Deref for OurU256 {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uint256> for OurU256 {
    fn from(value: Uint256) -> Self {
        Self(value.to_be_bytes())
    }
}

struct OurCrossChainId {
    chain: String,
    id: String,
}

impl From<CrossChainId> for OurCrossChainId {
    fn from(value: CrossChainId) -> Self {
        Self {
            chain: value.chain.into(),
            id: value.id.into(),
        }
    }
}

struct OurMessage {
    cc_id: OurCrossChainId,
    source_address: String,
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
}

impl From<Message> for OurMessage {
    fn from(value: Message) -> Self {
        Self {
            cc_id: value.cc_id.into(),
            source_address: value.source_address.deref().clone(),
            destination_chain: value.destination_chain.into(),
            destination_address: value.destination_address.deref().clone(),
            payload_hash: value.payload_hash,
        }
    }
}

struct OurSigner {
    address: String,
    weight: OurU256,
    public_key: OurPublicKey,
}

impl TryFrom<Signer> for OurSigner {
    type Error = StdError;

    fn try_from(value: Signer) -> Result<Self, Self::Error> {
        Ok(Self {
            address: value.address.into_string(),
            weight: value.weight.into(),
            public_key: value.pub_key.try_into()?,
        })
    }
}

struct OurWorkerSet {
    signers: BTreeMap<String, OurSigner>,
    threshold: OurU256,
    created_at: u64,
}

impl TryFrom<WorkerSet> for OurWorkerSet {
    type Error = StdError;

    fn try_from(value: WorkerSet) -> Result<Self, Self::Error> {
        let mut signers = BTreeMap::new();
        for (address, signer) in value.signers.into_iter() {
            let signer = OurSigner::try_from(signer)?;
            signers.insert(address, signer);
        }
        Ok(Self {
            signers,
            threshold: value.threshold.into(),
            created_at: value.created_at,
        })
    }
}

enum OurPayload {
    Messages(Vec<OurMessage>),
    WorkerSet(OurWorkerSet),
}

impl TryFrom<Payload> for OurPayload {
    type Error = StdError;

    fn try_from(value: Payload) -> Result<Self, Self::Error> {
        Ok(match value {
            Payload::Messages(messages) => {
                OurPayload::Messages(messages.into_iter().map(OurMessage::from).collect())
            }
            Payload::WorkerSet(worker_set) => OurPayload::WorkerSet(worker_set.try_into()?),
        })
    }
}

struct WeightedSignature {
    pubkey: OurPublicKey,
    signature: OurSignature,
    weight: OurU256,
}

impl From<SignerWithSig> for WeightedSignature {
    fn from(value: SignerWithSig) -> Self {
        Self {
            pubkey: value.signer.pub_key.try_into().unwrap(),
            signature: value.signature.try_into().unwrap(),
            weight: value.signer.weight.into(),
        }
    }
}

struct Proof {
    signatures: Vec<WeightedSignature>,
    threshold: Uint256,
    nonce: u64,
}

impl Proof {
    fn new(
        worker_set: &WorkerSet,
        mut signers_with_sigs: Vec<SignerWithSig>,
    ) -> Result<Self, ProofError> {
        // Signatures are sorted in ascending order
        signers_with_sigs.sort_by(|a, b| a.signer.pub_key.cmp(&b.signer.pub_key));

        let mut signatures: Vec<WeightedSignature> = Vec::with_capacity(signers_with_sigs.len());
        for signer in signers_with_sigs.into_iter() {
            // Check if the worker set contains this signer.
            if !worker_set
                .signers
                .contains_key(signer.signer.address.as_str())
            {
                return Err(ProofError::SignerNotInWorkerSet(
                    signer.signer.address.to_string(),
                ));
            }
            signatures.push(signer.into())
        }

        Ok(Proof {
            signatures,
            threshold: worker_set.threshold,
            nonce: worker_set.created_at,
        })
    }
}

struct ExecuteData {
    proof: Proof,
    payload: OurPayload,
}

pub fn encode(
    worker_set: &WorkerSet,
    mut signers: Vec<SignerWithSig>,
    payload_digest: &Hash,
    payload: &Payload,
) -> Result<HexBinary, ContractError> {
    for signer in &mut signers {
        recover_signature(payload_digest, signer)?;
    }

    let proof = Proof::new(worker_set, signers)?;
    let payload: OurPayload = payload.clone().try_into()?;
    let execute_data = ExecuteData { proof, payload };

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

    let mut recoverable_bytes = Vec::with_capacity(Recoverable::LEN);
    recoverable_bytes.extend(signature.to_bytes().iter());
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
