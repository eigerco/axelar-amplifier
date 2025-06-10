use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Request error: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("url: {0}")]
    Url(#[from] url::ParseError),
    #[error("Request error")]
    Request,
    #[error("Transaction '{0}' not found")]
    TransactionNotFound(String),
    #[error("Transition '{0}' not found")]
    TransitionNotFound(String),
    #[error("Failed to find callContract")]
    CallContractNotFound,
    #[error("Failed to find signerRotation")]
    SignerRotationNotFound,
    #[error("Failed to find user call")]
    UserCallnotFound,
    #[error("The program name is invalid: {0}")]
    InvalidProgramName(String),
    #[error("The provided chain name is invalid")]
    InvalidChainName,
    #[error("Invalid source address")]
    InvalidSourceAddress,
    #[error("Failed to create hash payload: {0}")]
    PayloadHash(String),
    #[error("Failed to find transition '{0}' in transaction")]
    TransitionNotFoundInTransaction(String),
    #[error("Failed to convert aleo string to json")]
    JsonParse(String),
    #[error("Failed to create CallContract receipt: {0}")]
    CalledContractReceipt(String),
    #[error("More than one SCM found")]
    MoreThanOneSCM,
    #[error("Call contract does not match")]
    CallConractDoesNotMatch,
    #[error("Deseirialization error: {0}")]
    DeserializationError(#[from] serde_aleo::Error),
    #[error("Axelar nonempty error: {0}")]
    NonemptyError(#[from] axelar_wasm_std::nonempty::Error),
    #[error("Router API error: {0}")]
    RouterApiError(#[from] router_api::error::Error),
}
