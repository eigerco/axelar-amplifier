use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid starknet address")]
    InvalidAddress,
}

/// Errors that can occur when processing events.
///
/// This enum contains all possible error cases that may arise during event
/// processing, including validation errors, parsing failures, and missing data
/// errors.
#[derive(Error, Debug)]
pub(crate) enum Parse {
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
    /// Contains the transaction identifier and the specific error message.
    #[error("failed to parse payload data for transaction: {0}, error: {1}")]
    FailedToParsePayloadData(String, String),

    /// Error returned when the destination address cannot be parsed.
    // /// Contains the transaction identifier and the specific ByteArray error.
    // #[error("failed to parse destination address for transaction: {0}, error: {1}")]
    // FailedToParseDestinationAddress(String, ByteArrayErrors),

    /// Error returned when required payload data is missing from a transaction.
    #[error("missing payload data for transaction: {0}")]
    MissingPayloadData(String),

    /// Error returned when payload data parsing fails due to ArraySpan-related
    /// issues. Contains the transaction identifier and the specific
    /// ArraySpan error.
    // #[error("failed to parse payload data for transaction: {0}, error: {1}")]
    // PayloadDataParsingFailed(String, ArraySpanErrors),

    /// Error returned when there are no events available for processing.
    #[error("no events to process")]
    NoEventsToProcess,

    /// Error returned when the command ID in a transaction is invalid or
    /// unexpected.
    #[error("incorrect command id for transaction: {0}")]
    IncorrectCommandId(String),

    // /// Error returned when the source chain information cannot be parsed.
    // /// Contains the transaction identifier and the specific ByteArray error.
    // #[error("failed to parse source chain for transaction: {0}, error: {1}")]
    // FailedToParseSourceChain(String, ByteArrayErrors),
    /// Error returned when the message ID cannot be parsed.
    /// Contains the transaction identifier and the specific ByteArray error.
    // #[error("failed to parse message id for transaction: {0}, error: {1}")]
    // FailedToParseMessageId(String, ByteArrayErrors),

    // /// Error returned when the source address cannot be parsed.
    // /// Contains the transaction identifier and the specific ByteArray error.
    // #[error("failed to parse source address for transaction: {0}, error: {1}")]
    // FailedToParseSourceAddress(String, ByteArrayErrors),

    /// Error returned when a contract address cannot be parsed from a
    /// transaction.
    #[error("failed to parse contract address for transaction: {0}")]
    FailedToParseContractAddress(String),

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
}
