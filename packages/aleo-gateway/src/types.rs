mod safe_gmp_chain_name;
pub use safe_gmp_chain_name::*;

/// Used for chain names used in Aleo GMP
pub type GmpChainName = [u128; 2];

/// Used for message ids used in Aleo GMP
pub type GmpMessageId = [u128; 8];

/// Used for addresses used in Aleo GMP
pub type GmpAddress = [u128; 6];

/// ITS token id used in Aleo ITS
pub type ItsTokenId = [u128; 2];
