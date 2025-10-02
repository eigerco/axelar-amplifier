use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StringEncoderError(#[from] aleo_string_encoder::AleoStringEncoderError),
    #[error("Invalid chain name: {0}")]
    InvalidChainName(String),
    #[error(transparent)]
    AxelarInvalidChainName(#[from] axelar_wasm_std::chain::Error),
    #[error(transparent)]
    SnarkVM(#[from] snarkvm_cosmwasm::prelude::Error),
    #[error("Utf8Error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("ConversionOverflowError: {0}")]
    ConversionOverflow(#[from] cosmwasm_std::ConversionOverflowError),
    #[error("Invalid token manager type")]
    InvalidTokenManagerType,
    #[error("Invalid operator address")]
    InvalidOperatorAddress,
}
