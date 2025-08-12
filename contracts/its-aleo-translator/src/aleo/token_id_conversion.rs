use std::ops::{Deref, Index};

use interchain_token_service_std::TokenId;

/// Internal wrapper for converting between ITS TokenId and Aleo's u128 pair format.
///
/// ITS uses 32-byte token IDs for cross-chain compatibility, while Aleo programs
/// work with pairs of u128 values. This newtype provides bidirectional conversion
/// between these formats while preserving the original token ID semantics.
///
/// ## Format Details
///
/// - **ITS Format**: 32 bytes (256 bits) as `[u8; 32]`
/// - **Aleo Format**: 2 × u128 values as `[u128; 2]`
/// - **Conversion**: Big-endian byte order is used to maintain consistency
///
/// The conversion splits the 32 bytes into two 16-byte chunks, each converted
/// to a u128 in big-endian format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ItsTokenIdNewType(pub [u128; 2]);

impl From<ItsTokenIdNewType> for TokenId {
    /// Converts from Aleo's u128 pair format back to ITS TokenId.
    ///
    /// Reconstructs the original 32-byte token ID by converting each u128
    /// to big-endian bytes and concatenating them.
    fn from(value: ItsTokenIdNewType) -> Self {
        let mut result = [0u8; 32];
        result[0..16].copy_from_slice(&value.0[0].to_be_bytes());
        result[16..32].copy_from_slice(&value.0[1].to_be_bytes());

        TokenId::from(result)
    }
}

impl From<TokenId> for ItsTokenIdNewType {
    /// Converts from ITS TokenId to Aleo's u128 pair format.
    ///
    /// Splits the 32-byte token ID into two u128 values for Aleo compatibility.
    /// Uses big-endian byte order to ensure consistent conversion.
    fn from(value: TokenId) -> Self {
        let input: [u8; 32] = value.into();
        // Safe to unwrap: input is guaranteed to be exactly 32 bytes, so slicing by 16 elements
        // twice makes 'try_into()' infallible.
        #[allow(clippy::unwrap_used)]
        let result = [
            u128::from_be_bytes(input[0..16].try_into().unwrap()),
            u128::from_be_bytes(input[16..32].try_into().unwrap()),
        ];

        ItsTokenIdNewType(result)
    }
}

impl Index<usize> for ItsTokenIdNewType {
    type Output = u128;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Deref for ItsTokenIdNewType {
    type Target = [u128; 2];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
