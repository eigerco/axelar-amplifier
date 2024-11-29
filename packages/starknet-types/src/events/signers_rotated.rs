use starknet_core::types::{Event, Felt};
use thiserror::Error;

/// Errors that can occur when processing events.
///
/// This enum contains all possible error cases that may arise during event
/// processing, including validation errors, parsing failures, and missing data
/// errors.
#[derive(Error, Debug)]
pub enum Parse {
    /// Error returned when the sender address in a transaction is invalid or
    /// unexpected.
    #[error("incorrect sender address for transaction: {0}")]
    IncorrectSenderAddress(String),

    /// Error returned when the destination chain specified in a transaction is
    /// invalid.
    #[error("incorrect destination chain for transaction: {0}")]
    IncorrectDestinationChain(String),

    /// Error returned when the destination contract address in a transaction is
    /// invalid.
    #[error("incorrect destination contract address for transaction: {0}")]
    IncorrectDestinationContractAddress(String),

    /// Error returned when a required payload hash is missing from a
    /// transaction.
    #[error("missing payload hash for transaction: {0}")]
    MissingPayloadHash(String),

    /// Error returned when a required signers hash is missing from a
    /// transaction.
    #[error("missing signers hash for transaction: {0}")]
    MissingSignersHash(String),

    /// Error returned when payload data cannot be parsed correctly.
    #[error("failed to parse payload data for transaction: {0}, error: {1}")]
    FailedToParsePayloadData(String, String),

    /// Error returned when the payload data is missing.
    #[error("missing payload data for transaction: {0}")]
    MissingPayloadData(String),

    /// Error returned when there are no events available for processing.
    #[error("no events to process")]
    NoEventsToProcess,

    /// Error returned when the command ID in a transaction is invalid or
    /// unexpected.
    #[error("incorrect command id for transaction: {0}")]
    IncorrectCommandId(String),

    /// Error returned when the epoch number in a transaction is invalid or
    /// unexpected.
    #[error("incorrect epoch for transaction: {0}")]
    IncorrectEpoch(String),

    /// Error returned when the threshold in a transaction is invalid or
    /// unexpected.
    #[error("incorrect threshold for transaction: {0}")]
    IncorrectThreshold(String),

    /// Error returned when the nonce in a transaction is missing.
    #[error("missing nonce for transaction: {0}")]
    MissingNonce(String),

    /// Error returned when the keys in a transaction are missing.
    #[error("missing keys for transaction: {0}")]
    MissingKeys(String),
}

/// Represents an Ethereum address as a fixed-length byte array
///
/// An Ethereum address is a 20-byte (40 hex character) identifier that
/// represents an account or contract on the Ethereum blockchain. This struct
/// provides a type-safe way to handle Ethereum addresses with validation and
/// conversion methods.
#[derive(Clone, PartialEq, Debug, PartialOrd, Copy, Eq)]
pub struct Address([u8; Self::ETH_ADDRESS_LEN]);

impl Address {
    /// The length of an Ethereum address in bytes (20 bytes = 40 hex
    /// characters)
    pub const ETH_ADDRESS_LEN: usize = 20;
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<[u8]> for Address {
    fn eq(&self, other: &[u8]) -> bool {
        self.0 == other
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = Parse;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; Self::ETH_ADDRESS_LEN] = bytes.try_into().map_err(|_| {
            Parse::FailedToParsePayloadData(
                "failed to parse signer address".to_string(),
                "failed to parse signer address".to_string(),
            )
        })?;
        Ok(Self(bytes))
    }
}

impl From<[u8; Self::ETH_ADDRESS_LEN]> for Address {
    fn from(value: [u8; Self::ETH_ADDRESS_LEN]) -> Self {
        Self(value)
    }
}

/// Represents a weighted signer in the Starknet gateway
///
/// A weighted signer consists of:
/// * A signer address
/// * A weight value representing their voting power
///
/// The weight is used when calculating if a threshold is met for
/// multi-signature operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signer {
    /// The address of the signer
    pub signer: Address,
    /// The weight (voting power) of this signer
    pub weight: u128,
}

/// Represents a set of weighted signers
/// TODO: reason why we need this struct, can't we just import from
/// packages/evm_gateway crate?
#[derive(Debug, Clone)]
pub struct WeightedSigners {
    pub signers: Vec<Signer>,
    pub threshold: u128,
    pub nonce: [u8; 32],
}

/// Represents a Starknet SignersRotated event.
#[derive(Debug, Clone)]
pub struct SignersRotated {
    /// The transaction hash
    pub tx_signature: Felt,
    /// The epoch number when this rotation occurred
    pub epoch: u64,
    /// The hash of the new signers
    pub signers_hash: [u8; 32],
    /// The new set of weighted signers with their voting power
    pub signers: WeightedSigners,
}

impl SignersRotated {
    /// Parses a Starknet SignersRotated event from a given event.
    ///
    /// This function extracts the relevant information from the event and constructs
    /// a `SignersRotated` struct.
    pub fn parse(event: Event, tx_hash: Felt) -> Result<Self, Parse> {
        if event.data.is_empty() {
            return Err(Parse::MissingPayloadData(tx_hash.to_string()));
        }
        if event.keys.is_empty() {
            return Err(Parse::MissingKeys(tx_hash.to_string()));
        }

        // To avoid multiple convertions
        let tx_hash_str = tx_hash.to_string();

        // it starts at 2 because 0 is the selector and 1 is the from_address
        let epoch_index = 2;
        // INFO: there might be better way to convert to u64
        let epoch = event
            .keys
            .get(epoch_index)
            .ok_or(Parse::IncorrectEpoch(tx_hash_str.clone()))?
            .to_string()
            .parse::<u64>()
            .map_err(|_| Parse::IncorrectEpoch(tx_hash_str.clone()))?;

        // Construct signers hash
        let mut signers_hash = [0_u8; 32];
        let lsb = event
            .keys
            .get(epoch_index + 1)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| Parse::MissingSignersHash(tx_hash_str.clone()))?;
        let msb = event
            .keys
            .get(epoch_index + 2)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| Parse::MissingSignersHash(tx_hash_str.clone()))?;
        signers_hash[..16].copy_from_slice(&msb[16..]);
        signers_hash[16..].copy_from_slice(&lsb[16..]);

        // Parse signers array from event data
        let mut buff_signers = vec![];

        let signers_index = 0;
        let signers_len = event.data[signers_index]
            .to_string()
            .parse::<usize>()
            .map_err(|_| {
                Parse::FailedToParsePayloadData(
                    tx_hash_str.clone(),
                    "failed to parse signers length".to_string(),
                )
            })?;
        let signers_end_index = signers_index + signers_len * 2;

        // Parse signers and weights
        for i in 0..signers_len {
            let signer_index = signers_index + 1 + (i * 2);
            let weight_index = signer_index + 1;

            // Get signer address as bytes
            let signer_bytes = event.data[signer_index].to_bytes_be();

            // Create Address from bytes, skipping first 12 bytes since Address is 20 bytes
            let signer = Address::try_from(&signer_bytes[12..]).map_err(|_| {
                Parse::FailedToParsePayloadData(
                    tx_hash_str.clone(),
                    "failed to parse signer address".to_string(),
                )
            })?;

            // Parse weight
            let weight = event.data[weight_index]
                .to_string()
                .parse::<u128>()
                .map_err(|_| {
                    Parse::FailedToParsePayloadData(
                        tx_hash_str.clone(),
                        "failed to parse signer weight".to_string(),
                    )
                })?;

            buff_signers.push(Signer { signer, weight });
        }

        // Parse threshold
        let threshold = event
            .data
            .get(signers_end_index)
            .ok_or_else(|| Parse::IncorrectThreshold(tx_hash_str.clone()))?
            .to_string()
            .parse::<u128>()
            .map_err(|_| Parse::IncorrectThreshold(tx_hash_str.clone()))?;

        // Parse nonce
        let mut nonce = [0_u8; 32];
        let lsb = event
            .keys
            .get(signers_end_index + 1)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| Parse::MissingNonce(tx_hash_str.clone()))?;
        let msb = event
            .keys
            .get(signers_end_index + 2)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| Parse::MissingNonce(tx_hash_str.clone()))?;
        nonce[..16].copy_from_slice(&msb[16..]);
        nonce[16..].copy_from_slice(&lsb[16..]);

        Ok(SignersRotated {
            tx_signature: tx_hash,
            epoch,
            signers_hash,
            signers: WeightedSigners {
                signers: buff_signers,
                threshold,
                nonce,
            },
        })
    }
}
