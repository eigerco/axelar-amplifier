//! Error variants produced by Validator and it's methods
//! Use it to implement proper error handling on consumer's side

use thiserror::Error;

/// Error enumerator
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    JsonDeserializeError(#[from] serde_json::Error),
}
