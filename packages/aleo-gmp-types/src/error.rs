use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StringEncoderError(#[from] aleo_string_encoder::AleoStringEncoderError),
    #[error("Invalid chain name: {0}")]
    InvalidChainName(String),
    #[error(transparent)]
    AxelarInvalidChainName(#[from] axelar_wasm_std::chain::Error),
}
