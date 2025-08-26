use cosmwasm_std::Uint256;
use multisig::msg::{Signer, SignerWithSig};
use multisig::verifier_set::VerifierSet;
use starknet_crypto::poseidon_hash_many;
use starknet_message::CairoSerialize;
use starknet_types_core::felt::Felt;

use crate::error::ContractError;
use crate::payload::Payload;

/// STARK signature with 3 felts (r, s, v)
#[derive(Clone, Debug, PartialEq)]
pub struct StarkSignature {
    pub r: Felt,
    pub s: Felt,
    pub v: Felt, // Recovery ID as felt (0 or 1)
}

/// Starknet-compatible WeightedSigner
#[derive(Clone, Debug, PartialEq)]
pub struct StarknetWeightedSigner {
    pub signer: Felt, // STARK public key as felt252
    pub weight: u128,
}

/// Starknet-compatible WeightedSigners
#[derive(Clone, Debug, PartialEq)]
pub struct StarknetWeightedSigners {
    pub signers: Vec<StarknetWeightedSigner>,
    pub threshold: u128,
    pub nonce: [u8; 32], // U256 as bytes to serialize as low/high
}

/// Starknet-compatible Proof
#[derive(Clone, Debug, PartialEq)]
pub struct StarknetProof {
    pub signers: StarknetWeightedSigners,
    pub signatures: Vec<StarkSignature>,
}

/// Command types for Starknet flow
#[derive(Clone, Debug, PartialEq)]
pub enum StarknetCommandType {
    ApproveMessages,
    RotateSigners,
}

impl CairoSerialize for StarknetCommandType {
    fn cairo_serialize(&self) -> Vec<Felt> {
        match self {
            StarknetCommandType::ApproveMessages => vec![Felt::ZERO],
            StarknetCommandType::RotateSigners => vec![Felt::ONE],
        }
    }
}

impl CairoSerialize for StarkSignature {
    fn cairo_serialize(&self) -> Vec<Felt> {
        vec![self.r, self.s, self.v]
    }
}

impl CairoSerialize for StarknetWeightedSigner {
    fn cairo_serialize(&self) -> Vec<Felt> {
        vec![self.signer, Felt::from(self.weight)]
    }
}

impl CairoSerialize for StarknetWeightedSigners {
    fn cairo_serialize(&self) -> Vec<Felt> {
        let mut elements = Vec::new();

        // Signers array length
        elements.push(Felt::from(self.signers.len() as u32));

        // Each signer's data
        for signer in &self.signers {
            elements.extend(signer.cairo_serialize());
        }

        // Threshold
        elements.push(Felt::from(self.threshold));

        // Nonce as U256 - serialize as low, high (each 128 bits)
        // Take last 16 bytes as low, first 16 bytes as high (big-endian U256)
        let high_bytes = &self.nonce[..16];
        let low_bytes = &self.nonce[16..];

        // Convert to u128 values (big-endian)
        let high = u128::from_be_bytes(high_bytes.try_into().unwrap());
        let low = u128::from_be_bytes(low_bytes.try_into().unwrap());

        // Add as felts (low first, then high for U256 serialization)
        elements.push(Felt::from(low));
        elements.push(Felt::from(high));

        elements
    }
}

impl CairoSerialize for StarknetProof {
    fn cairo_serialize(&self) -> Vec<Felt> {
        let mut elements = Vec::new();

        elements.extend(self.signers.cairo_serialize());

        elements.push(Felt::from(self.signatures.len() as u32));

        for signature in &self.signatures {
            elements.extend(signature.cairo_serialize());
        }

        elements
    }
}

/// Calculate the hash of WeightedSigners using Poseidon
pub fn signer_hash_poseidon(signers: &StarknetWeightedSigners) -> Felt {
    let elements = signers.cairo_serialize();
    poseidon_hash_many(&elements)
}

/// Calculate the payload digest using Poseidon hash
pub fn payload_digest_poseidon(domain_separator: Felt, signer_hash: Felt, data_hash: Felt) -> Felt {
    poseidon_hash_many(&[domain_separator, signer_hash, data_hash])
}

/// Calculate data hash for payload using Poseidon
pub fn data_hash_poseidon(payload_elements: &[Felt]) -> Felt {
    poseidon_hash_many(payload_elements)
}

// Type conversion implementations
impl TryFrom<&VerifierSet> for StarknetWeightedSigners {
    type Error = error_stack::Report<ContractError>;

    fn try_from(verifier_set: &VerifierSet) -> std::result::Result<Self, Self::Error> {
        let mut signers = Vec::new();

        for signer in verifier_set.signers.values() {
            signers.push(StarknetWeightedSigner::try_from(signer)?);
        }

        // Sort by signer felt value for consistency
        signers.sort_by_key(|s| s.signer);

        // Convert created_at (u64) to U256 bytes representation
        let nonce = Uint256::from(verifier_set.created_at).to_be_bytes();

        Ok(StarknetWeightedSigners {
            signers,
            threshold: verifier_set.threshold.u128(),
            nonce,
        })
    }
}

impl TryFrom<&Signer> for StarknetWeightedSigner {
    type Error = error_stack::Report<ContractError>;

    fn try_from(signer: &Signer) -> std::result::Result<Self, Self::Error> {
        // Extract the public key bytes and convert to Felt
        let pub_key_bytes = match &signer.pub_key {
            multisig::key::PublicKey::Ecdsa(bytes) => {
                // ECDSA public keys are 33 bytes (compressed) or 65 bytes (uncompressed)
                // We'll use the first 32 bytes for the felt (excluding compression prefix if present)
                if bytes.len() == 33 {
                    &bytes.as_slice()[1..] // Skip compression byte
                } else if bytes.len() == 65 {
                    &bytes.as_slice()[1..33] // Skip prefix, take first 32 bytes
                } else {
                    bytes.as_slice()
                }
            }
            multisig::key::PublicKey::Ed25519(bytes) => {
                // Ed25519 public keys are 32 bytes
                bytes.as_slice()
            }
            multisig::key::PublicKey::Stark(bytes) => {
                // STARK public keys should already be in felt format
                bytes.as_slice()
            }
        };

        // Ensure we have exactly 32 bytes or less for felt conversion
        let signer_felt = if pub_key_bytes.len() <= 32 {
            let mut padded = [0u8; 32];
            padded[32 - pub_key_bytes.len()..].copy_from_slice(pub_key_bytes);
            Felt::from_bytes_be(&padded)
        } else {
            return Err(ContractError::InvalidPublicKey {
                reason: "Public key too large for felt conversion".to_string(),
            }
            .into());
        };

        Ok(StarknetWeightedSigner {
            signer: signer_felt,
            weight: signer.weight.u128(),
        })
    }
}

impl TryFrom<SignerWithSig> for StarkSignature {
    type Error = error_stack::Report<ContractError>;

    fn try_from(signer_with_sig: SignerWithSig) -> std::result::Result<Self, Self::Error> {
        let sig_bytes = signer_with_sig.signature.as_ref();

        // We only expect recoverable STARK signatures which are 96 bytes (r: 32, s: 32, v: 32)
        if sig_bytes.len() < 96 {
            return Err(ContractError::InvalidSignature {
                reason: "Signature too short".to_string(),
            }
            .into());
        }

        // Extract r and s from signature (first 32 bytes each)
        let mut r_bytes = [0u8; 32];
        r_bytes.copy_from_slice(&sig_bytes[0..32]);
        let r = Felt::from_bytes_be(&r_bytes);

        let mut s_bytes = [0u8; 32];
        s_bytes.copy_from_slice(&sig_bytes[32..64]);
        let s = Felt::from_bytes_be(&s_bytes);

        let mut v_bytes = [0u8; 32];
        v_bytes.copy_from_slice(&sig_bytes[64..96]);
        let v = Felt::from_bytes_be(&v_bytes);

        Ok(StarkSignature { r, s, v })
    }
}

impl From<&Payload> for StarknetCommandType {
    fn from(payload: &Payload) -> Self {
        match payload {
            Payload::Messages(_) => StarknetCommandType::ApproveMessages,
            Payload::VerifierSet(_) => StarknetCommandType::RotateSigners,
        }
    }
}
