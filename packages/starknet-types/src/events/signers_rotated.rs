use starknet_core::types::{Event, Felt};
use thiserror::Error;

/// An error, representing failure to convert/parse a starknet event
/// to a SignersRotated event.
#[derive(Error, Debug)]
pub enum SignersRotatedErrors {
    /// Error returned when a required signers hash is missing from a
    /// transaction.
    #[error("missing signers hash for transaction")]
    MissingSignersHash,

    /// Error returned when payload data cannot be parsed correctly.
    #[error("failed to parse payload data, error: {0}")]
    FailedToParsePayloadData(String),

    /// Error returned when the payload data is missing.
    #[error("missing payload data for transaction")]
    MissingPayloadData,

    /// Error returned when the epoch number in a transaction is invalid or
    /// unexpected.
    #[error("incorrect epoch for transaction")]
    IncorrectEpoch,

    /// Error returned when the threshold in a transaction is invalid or
    /// unexpected.
    #[error("incorrect threshold for transaction")]
    IncorrectThreshold,

    /// Error returned when the nonce in a transaction is missing.
    #[error("missing nonce for transaction")]
    MissingNonce,

    /// Error returned when the keys in a transaction are missing.
    #[error("missing keys for transaction")]
    MissingKeys,
}

/// Represents a weighted signer
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signer {
    /// The address of the signer
    pub signer: String,
    /// The weight (voting power) of this signer
    pub weight: u128,
}

/// Represents a set of signers
#[derive(Debug, Clone)]
pub struct WeightedSigners {
    pub signers: Vec<Signer>,
    pub threshold: u128,
    pub nonce: [u8; 32],
}

/// Represents a Starknet SignersRotated event
#[derive(Debug, Clone)]
pub struct SignersRotatedEvent {
    /// The address of the sender
    pub from_address: String,
    /// The epoch number when this rotation occurred
    pub epoch: u64,
    /// The hash of the new signers
    pub signers_hash: [u8; 32],
    /// The new set of weighted signers with their voting power
    pub signers: WeightedSigners,
}

impl TryFrom<starknet_core::types::Event> for SignersRotatedEvent {
    type Error = SignersRotatedErrors;

    /// Attempts to convert a Starknet event to a SignersRotated event
    ///
    /// # Arguments
    ///
    /// * `event` - The Starknet event to convert
    ///
    /// # Returns
    ///
    /// * `Ok(SignersRotated)` - Successfully converted event containing:
    ///   * `epoch` - The epoch number when rotation occurred
    ///   * `signers_hash` - Hash of the new signers (32 bytes)
    ///   * `signers` - New set of weighted signers with:
    ///     * List of signers with their addresses and weights
    ///     * Threshold for required voting power
    ///     * Nonce value (32 bytes)
    ///
    /// # Errors
    ///
    /// Returns a `SignersRotatedErrors` if:
    /// * Event data or keys are empty
    /// * Failed to parse epoch number
    /// * Missing or invalid signers hash
    /// * Failed to parse signers array length
    /// * Failed to parse signer addresses or weights
    /// * Missing or invalid threshold
    /// * Missing or invalid nonce
    fn try_from(event: Event) -> Result<Self, Self::Error> {
        if event.data.is_empty() {
            return Err(SignersRotatedErrors::MissingPayloadData);
        }
        if event.keys.is_empty() {
            return Err(SignersRotatedErrors::MissingKeys);
        }

        let from_address = event.from_address.to_hex_string();

        // it starts at 2 because 0 is the selector and 1 is the from_address
        let epoch_index = 2;
        // INFO: there might be better way to convert to u64
        let epoch = event
            .keys
            .get(epoch_index)
            .ok_or(SignersRotatedErrors::IncorrectEpoch)?
            .to_string()
            .parse::<u64>()
            .map_err(|_| SignersRotatedErrors::IncorrectEpoch)?;

        // Construct signers hash
        let mut signers_hash = [0_u8; 32];
        let lsb = event
            .keys
            .get(epoch_index + 1)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| SignersRotatedErrors::MissingSignersHash)?;
        let msb = event
            .keys
            .get(epoch_index + 2)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| SignersRotatedErrors::MissingSignersHash)?;
        signers_hash[..16].copy_from_slice(&msb[16..]);
        signers_hash[16..].copy_from_slice(&lsb[16..]);

        // Parse signers array from event data
        let mut buff_signers = vec![];

        let signers_index = 0;
        let signers_len = event.data[signers_index]
            .to_string()
            .parse::<usize>()
            .map_err(|_| {
                SignersRotatedErrors::FailedToParsePayloadData(
                    "failed to parse signers length".to_string(),
                )
            })?;
        let signers_end_index = signers_index + signers_len * 2;

        // Parse signers and weights
        for i in 0..signers_len {
            let signer_index = signers_index + 1 + (i * 2);
            let weight_index = signer_index + 1;

            // Get signer address as bytes
            let signer = event.data[signer_index].to_hex_string();

            // Parse weight
            let weight = event.data[weight_index]
                .to_string()
                .parse::<u128>()
                .map_err(|_| {
                    SignersRotatedErrors::FailedToParsePayloadData(
                        "failed to parse signer weight".to_string(),
                    )
                })?;

            buff_signers.push(Signer { signer, weight });
        }

        // Parse threshold
        let threshold = event
            .data
            .get(signers_end_index)
            .ok_or_else(|| SignersRotatedErrors::IncorrectThreshold)?
            .to_string()
            .parse::<u128>()
            .map_err(|_| SignersRotatedErrors::IncorrectThreshold)?;

        // Parse nonce
        let mut nonce = [0_u8; 32];
        let lsb = event
            .keys
            .get(signers_end_index + 1)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| SignersRotatedErrors::MissingNonce)?;
        let msb = event
            .keys
            .get(signers_end_index + 2)
            .map(Felt::to_bytes_be)
            .ok_or_else(|| SignersRotatedErrors::MissingNonce)?;
        nonce[..16].copy_from_slice(&msb[16..]);
        nonce[16..].copy_from_slice(&lsb[16..]);

        Ok(SignersRotatedEvent {
            from_address,
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
