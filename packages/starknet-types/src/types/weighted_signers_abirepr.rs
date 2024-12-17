use error_stack::{Report, ResultExt};
use ethers_core::abi::{InvalidOutputType, Token, Tokenizable};
use ethers_core::types::U256;
use starknet_core::types::Felt;

use crate::error::Error;

/// [`WeightedSignersAbiRep`] consist of public keys of signers, weights (bond)
/// desired threshold and nonce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedSignersAbiRep {
    /// List of signer addresses, each represented as a 20-byte array
    pub signers: Vec<[u8; 20]>,
    /// Minimum weight threshold required
    pub threshold: u128,
    /// To prevent replay attacks and collisions
    pub nonce: [u8; 32],
}

impl Tokenizable for WeightedSignersAbiRep {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        if let Token::Tuple(tokens) = token {
            if tokens.len() != 3 {
                return Err(InvalidOutputType(
                    "failed to read tokens; not enough tokens provided; should be 3".to_string(),
                ));
            }

            if let (Token::Array(signers), Token::Uint(threshold), Token::FixedBytes(nonce)) =
                (tokens[0].clone(), tokens[1].clone(), tokens[2].clone())
            {
                return Ok(WeightedSignersAbiRep {
                    // FIXME: fix this
                    signers: signers.into_iter().map(|s| [0_u8; 20]).collect(),
                    threshold: threshold.as_u128(),
                    nonce: nonce.try_into().unwrap(),
                });
            }
        }

        return Err(InvalidOutputType(
            "failed to convert tokens to StarknetMessage".to_string(),
        ));
    }

    fn into_token(self) -> Token {
        Token::Tuple(vec![])
    }
}

//
// https://github.com/axelarnetwork/axelar-amplifier/blob/main/contracts/multisig-prover/src/encoding/abi.rs#L217
//
// #[test]
//     fn abi_rotate_signers_execute_data() {
//         let domain_separator = domain_separator();

//         let new_pub_keys = vec![
//
// "0352a321079b435a4566ac8c92ab18584d8537d563f6c2c0bbbf58246ad047c611",
//
// "03b80cd1fff796fb80a82f4d45b812451668791a85a58c8c0b5939d75f126f80b1",
//
// "0251f7035a693e804eaed139009ede4ef62215914ccf9080027d53ef6bbb8897c5",
//
// "03a907596748daa5ae9c522445529ca38d0ea2c47a908c30643ca37a0e6e12160d",
//
// "03c55d66787c66f37257ef4b320ddcb64623d59e9bf0f3ec0f8ac7311b70cdd2c8",
//         ];

//         let mut new_verifier_set = verifier_set_from_pub_keys(new_pub_keys);
//         new_verifier_set.created_at = 2024;

//         let verifier_set = curr_verifier_set();

//         // Generated signatures are already sorted by weight and evm address
//         let sigs: Vec<_> = vec![
//
// "e3a7c09bfa26df8bbd207df89d7ba01100b809324b2987e1426081284a50485345a5a20b6d1d5844470513099937f1015ce8f4832d3df97d053f044103434d8c1b"
// ,
// "895dacfb63684da2360394d5127696129bd0da531d6877348ff840fb328297f870773df3c259d15dd28dbd51d87b910e4156ff2f3c1dc5f64d337dea7968a9401b"
// ,
// "7c685ecc8a42da4cd9d6de7860b0fddebb4e2e934357500257c1070b1a15be5e27f13b627cf9fa44f59d535af96be0a5ec214d988c48e2b5aaf3ba537d0215bb1b"
// ,         ].into_iter().map(|sig|
// HexBinary::from_hex(sig).unwrap()).collect();

//         let signers_with_sigs =
// signers_with_sigs(verifier_set.signers.values(), sigs);

//         let payload = Payload::VerifierSet(new_verifier_set);

//         let execute_data = assert_ok!(encode_execute_data(
//             &domain_separator,
//             &verifier_set,
//             signers_with_sigs,
//             &payload
//         ));

//         goldie::assert!(execute_data.to_hex());
//     }
