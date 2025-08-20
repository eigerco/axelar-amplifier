use axelar_wasm_std::IntoContractError;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, IntoContractError)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Invalid signature length: expected 96 bytes, got {0} bytes")]
    InvalidSignatureLength(usize),

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Signature verification failed")]
    VerificationFailed,
}
