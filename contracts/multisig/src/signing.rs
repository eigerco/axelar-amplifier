use std::collections::HashMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary, Uint128, Uint64};
use router_api::ChainName;
use signature_verifier_api::client::SignatureVerifier;

use crate::key::{PublicKey, Signature};
use crate::types::{MsgToSign, MultisigState};
use crate::verifier_set::VerifierSet;
use crate::ContractError;

#[cw_serde]
pub struct SigningSession {
    pub id: Uint64,
    pub verifier_set_id: String,
    pub chain_name: ChainName,
    pub msg: MsgToSign,
    pub state: MultisigState,
    pub expires_at: u64,
    pub sig_verifier: Option<Addr>,
}

impl SigningSession {
    pub fn new(
        session_id: Uint64,
        verifier_set_id: String,
        chain_name: ChainName,
        msg: MsgToSign,
        expires_at: u64,
        sig_verifier: Option<Addr>,
    ) -> Self {
        Self {
            id: session_id,
            verifier_set_id,
            chain_name,
            msg,
            state: MultisigState::Pending,
            expires_at,
            sig_verifier,
        }
    }

    pub fn recalculate_session_state(
        &mut self,
        signatures: &HashMap<String, Signature>,
        verifier_set: &VerifierSet,
        block_height: u64,
    ) {
        let weight = signers_weight(signatures, verifier_set);

        if self.state == MultisigState::Pending && weight >= verifier_set.threshold {
            self.state = MultisigState::Completed {
                completed_at: block_height,
            };
        }
    }
}

pub fn validate_session_signature(
    session: &SigningSession,
    signer: &Addr,
    signature: &Signature,
    pub_key: &PublicKey,
    block_height: u64,
    sig_verifier: Option<SignatureVerifier>,
) -> Result<(), ContractError> {
    if session.expires_at < block_height {
        return Err(ContractError::SigningSessionClosed {
            session_id: session.id,
        });
    }

    sig_verifier
        .map_or_else(
            || signature.verify(&session.msg, pub_key),
            |sig_verifier| {
                call_sig_verifier(
                    sig_verifier,
                    signature.as_ref().into(),
                    session.msg.as_ref().into(),
                    pub_key.as_ref().into(),
                    signer.to_string(),
                    session.id,
                )
            },
        )
        .map_err(|e| ContractError::InvalidSignature {
            session_id: session.id,
            signer: signer.into(),
            reason: e.to_string(),
        })?;

    Ok(())
}

/*
status: Unknown,
message: "failed to execute message; message index: 0: \"axelar1ckguw8l9peg6sykx30cy35t6l0wpfu23xycme7\" submitted an invalid signature for signing session Uint64(115), reason \"Generic error:
LOOK:
address: 'aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak',
message: '[734426967266271119157240286202784854182177084087311079640681941965241316364field, 211744439field]',
signature: 'sign1jfnmrylk3kk6ns2s355mxdhaq8l4yfrp4eq3dqyz2ckgtzy5csppefuayajhxea7hrasnmnlucs3xd5xa5phpx3aqs8d3jlxtnpnsqvpxp6apcrgnzaw3gfdywla9m4vxywvhnsuewd38alswp3pxmu8zt6pj54ng8w2txvnzjp5c3pyu9lt54f6hlgxuln98jgzrwnpsqassch0a80',
hex_binary: 'HexBinary(5415341bb3755cc8851e6e90738243d739511001aebe587f8ff0ea67dcda7b12)'\"
: execute wasm contract failed [CosmWasm/wasmd@v0.34.1/x/wasm/keeper/keeper.go:386] With gas wanted: '0' and gas used: '395966' ", details: [], metadata: MetadataMap { headers: {"content-type": "application/grpc", "x-cosmos-block-height": "5612175"} }
*/
fn call_sig_verifier(
    sig_verifier: SignatureVerifier,
    signature: HexBinary,
    message: HexBinary,
    pub_key: HexBinary,
    signer: String,
    session_id: Uint64,
) -> Result<(), ContractError> {
    let res = sig_verifier
        .verify_signature(signature, message, pub_key, signer, session_id)
        .map_err(|err| ContractError::SignatureVerificationFailed {
            reason: err.to_string(),
        })?;

    // let res = verify_signature::<snarkvm_cosmwasm::network::TestnetV0>(signature, message, pub_key)?;
    if !res {
        Err(ContractError::SignatureVerificationFailed {
            reason: "unable to verify signature, verify signature is false".into(),
        })
    } else {
        Ok(())
    }
}

fn signers_weight(signatures: &HashMap<String, Signature>, verifier_set: &VerifierSet) -> Uint128 {
    signatures
        .keys()
        .map(|addr| -> Uint128 {
            verifier_set
                .signers
                .get(addr)
                .expect("violated invariant: signature submitted by non-participant")
                .weight
        })
        .sum()
}

use std::str::FromStr as _;

use cosmwasm_std::StdResult;
use snarkvm_cosmwasm::account::ToFields;
use snarkvm_cosmwasm::program::{Network, Value};
use snarkvm_cosmwasm::program::Signature as AleoSignature;


use snarkvm_cosmwasm::types::{Address, Field};

/*
*  LOOK: address: 'aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak',
*  message: '[3570993132320435925381642947470595733315932391169339455385687565127846720524field, 190585187field]',
*  signature: 'sign1jdgzp5enav4uk3n9ueld9lrh4h50pppc5p0hkxehsyn96j8k2cqfyelnsn3tluh4szpwdlrpgd3dxn7gu070m3qszwaalnswl6n3kqvpxp6apcrgnzaw3gfdywla9m4vxywvhnsuewd38alswp3pxmu8zt6pj54ng8w2txvnzjp5c3pyu9lt54f6hlgxuln98jgzrwnpsqasskt28cu'\"
*/
pub fn verify_signature<N: Network>(
    signature: HexBinary,
    message: HexBinary,
    public_key: HexBinary,
) -> StdResult<bool> {
    let signed = String::from_utf8(signature.into()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
    })?;

    let signature = AleoSignature::<N>::from_str(signed.as_str()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
    })?;

    let address = String::from_utf8(public_key.into()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse public key: {}", e))
    })?;

    let addr = Address::from_str(address.as_str()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse address: {}", e))
    })?;

    let hex_binary = message.clone();
    let message = aleo_encoded(&message)?;

    let res = signature.verify(&addr, message.as_slice());
    if res == false {
        return Err(cosmwasm_std::StdError::generic_err(format!(
            "LOOK: address: '{:?}', message: '{:?}', signature: '{:?}', hex_binary: '{:?}'",
            addr, message, signature, hex_binary
        )));
    }

    Ok(res)
}

fn aleo_encoded<N: Network>(data: &HexBinary) -> Result<Vec<Field<N>>, cosmwasm_std::StdError> {
    let num = cosmwasm_std::Uint256::from_le_bytes(data.as_slice().try_into().unwrap());
    let message = format!("{num}group");

    let g = Value::from_str(message.as_str())
        .map_err(|e| {
            cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
        })?;
    println!("---->g: {:?}", g);
        g.to_fields()
        .map_err(|e| {
            cosmwasm_std::StdError::generic_err(format!("Failed to parse signature: {}", e))
        })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{MockApi, MockQuerier};
    use cosmwasm_std::{to_json_binary, HexBinary, QuerierWrapper};
    // use multisig_aleo::contract::query::verify_signature;

    use super::*;
    use crate::key::KeyType;
    use crate::test::common::{build_verifier_set, ecdsa_test_data, ed25519_test_data};

    #[test]
    fn my_test() {
        let signature = HexBinary::from("sign1jdgzp5enav4uk3n9ueld9lrh4h50pppc5p0hkxehsyn96j8k2cqfyelnsn3tluh4szpwdlrpgd3dxn7gu070m3qszwaalnswl6n3kqvpxp6apcrgnzaw3gfdywla9m4vxywvhnsuewd38alswp3pxmu8zt6pj54ng8w2txvnzjp5c3pyu9lt54f6hlgxuln98jgzrwnpsqasskt28cu".as_bytes());
        let message = HexBinary::from_hex("5415341bb3755cc8851e6e90738243d739511001aebe587f8ff0ea67dcda7b12").unwrap();
        let public_key = HexBinary::from("aleo145tj9hqrnv3hqylrem6p7zjyxc2kryyp3hdm4ht48ntj3e5ttuxs9xs9ak".as_bytes());
        // let public_key = HexBinary::from("aleo1gjslmjhfage7vdmctyjldpdv8yeqw0uk5wmpejexnt05w823tupshxzgw7".as_bytes());

        verify_signature::<snarkvm_cosmwasm::network::TestnetV0>(signature, message, public_key).unwrap();

    }

    pub struct TestConfig {
        pub verifier_set: VerifierSet,
        pub session: SigningSession,
        pub signatures: HashMap<String, Signature>,
        pub key_type: KeyType,
    }

    fn ecdsa_setup() -> TestConfig {
        let signers = ecdsa_test_data::signers();

        let verifier_set_id = "subkey".to_string();
        let key_type = KeyType::Ecdsa;
        let verifier_set = build_verifier_set(KeyType::Ecdsa, &signers);

        let message: MsgToSign = ecdsa_test_data::message().try_into().unwrap();
        let expires_at = 12345;
        let session = SigningSession::new(
            Uint64::one(),
            verifier_set_id,
            "mock-chain".parse().unwrap(),
            message.clone(),
            expires_at,
            None,
        );

        let signatures: HashMap<String, Signature> = signers
            .iter()
            .map(|signer| {
                (
                    signer.address.clone().into_string(),
                    Signature::try_from((key_type, signer.signature.clone())).unwrap(),
                )
            })
            .collect();

        TestConfig {
            verifier_set,
            session,
            signatures,
            key_type,
        }
    }

    fn ed25519_setup() -> TestConfig {
        let signers = ed25519_test_data::signers();

        let verifier_set_id = "subkey".to_string();
        let key_type = KeyType::Ed25519;
        let verifier_set = build_verifier_set(key_type, &signers);

        let message: MsgToSign = ed25519_test_data::message().try_into().unwrap();
        let expires_at = 12345;
        let session = SigningSession::new(
            Uint64::one(),
            verifier_set_id,
            "mock-chain".parse().unwrap(),
            message.clone(),
            expires_at,
            None,
        );

        let signatures: HashMap<String, Signature> = signers
            .iter()
            .map(|signer| {
                (
                    signer.address.clone().into_string(),
                    Signature::try_from((key_type, signer.signature.clone())).unwrap(),
                )
            })
            .collect();

        TestConfig {
            verifier_set,
            session,
            signatures,
            key_type,
        }
    }

    #[test]
    fn correct_session_state() {
        for config in [ecdsa_setup(), ed25519_setup()] {
            let mut session = config.session;
            let verifier_set = config.verifier_set;
            let signatures = config.signatures;
            let block_height = 12345;

            session.recalculate_session_state(&HashMap::new(), &verifier_set, block_height);
            assert_eq!(session.state, MultisigState::Pending);

            session.recalculate_session_state(&signatures, &verifier_set, block_height);
            assert_eq!(
                session.state,
                MultisigState::Completed {
                    completed_at: block_height
                }
            );
        }
    }

    #[test]
    fn success_validation() {
        for config in [ecdsa_setup(), ed25519_setup()] {
            let session = config.session;
            let verifier_set = config.verifier_set;
            let signer = Addr::unchecked(config.signatures.keys().next().unwrap());
            let signature = config.signatures.values().next().unwrap();
            let pub_key = &verifier_set
                .signers
                .get(&signer.to_string())
                .unwrap()
                .pub_key;

            assert!(
                validate_session_signature(&session, &signer, signature, pub_key, 0, None).is_ok()
            );
        }
    }

    #[test]
    fn validation_through_signature_verifier_contract() {
        for config in [ecdsa_setup(), ed25519_setup()] {
            let session = config.session;
            let verifier_set = config.verifier_set;
            let signer = Addr::unchecked(config.signatures.keys().next().unwrap());
            let signature = config.signatures.values().next().unwrap();
            let pub_key = &verifier_set
                .signers
                .get(&signer.to_string())
                .unwrap()
                .pub_key;

            for verification in [true, false] {
                let mut querier = MockQuerier::default();
                querier.update_wasm(move |_| Ok(to_json_binary(&verification).into()).into());
                let sig_verifier = Some(SignatureVerifier {
                    address: MockApi::default().addr_make("verifier"),
                    querier: QuerierWrapper::new(&querier),
                });

                let result = validate_session_signature(
                    &session,
                    &signer,
                    signature,
                    pub_key,
                    0,
                    sig_verifier,
                );

                if verification {
                    assert!(result.is_ok());
                } else {
                    assert_eq!(
                        result.unwrap_err(),
                        ContractError::InvalidSignature {
                            session_id: session.id,
                            signer: signer.clone().into(),
                            reason: "unable to verify signature, verify signature is false".into()
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn success_validation_expiry_not_reached() {
        for config in [ecdsa_setup(), ed25519_setup()] {
            let session = config.session;
            let verifier_set = config.verifier_set;
            let signer = Addr::unchecked(config.signatures.keys().next().unwrap());
            let signature = config.signatures.values().next().unwrap();
            let block_height = 12340; // inclusive
            let pub_key = &verifier_set
                .signers
                .get(&signer.to_string())
                .unwrap()
                .pub_key;

            assert!(validate_session_signature(
                &session,
                &signer,
                signature,
                pub_key,
                block_height,
                None
            )
            .is_ok());
        }
    }

    #[test]
    fn signing_session_closed_validation() {
        for config in [ecdsa_setup(), ed25519_setup()] {
            let session = config.session;
            let verifier_set = config.verifier_set;
            let signer = Addr::unchecked(config.signatures.keys().next().unwrap());
            let signature = config.signatures.values().next().unwrap();
            let block_height = 12346;
            let pub_key = &verifier_set
                .signers
                .get(&signer.to_string())
                .unwrap()
                .pub_key;

            let result = validate_session_signature(
                &session,
                &signer,
                signature,
                pub_key,
                block_height,
                None,
            );

            assert_eq!(
                result.unwrap_err(),
                ContractError::SigningSessionClosed {
                    session_id: session.id,
                }
            );
        }
    }

    #[test]
    fn invalid_signature_validation() {
        for config in [ecdsa_setup(), ed25519_setup()] {
            let session = config.session;
            let verifier_set = config.verifier_set;
            let signer = Addr::unchecked(config.signatures.keys().next().unwrap());
            let pub_key = &verifier_set
                .signers
                .get(&signer.to_string())
                .unwrap()
                .pub_key;

            let sig_bytes = match config.key_type {
                KeyType::Ecdsa =>   "a58c9543b9df54578ec45838948e19afb1c6e4c86b34d9899b10b44e619ea74e19b457611e41a047030ed233af437d7ecff84de97cb6b3c13d73d22874e03511",
                KeyType::Ed25519 => "1fe264eb7258d48d8feedea4d237ccb20157fbe5eb412bc971d758d072b036a99b06d20853c1f23cdf82085917e08dda2fcfbb5d4d7ee17d74e4988ae81d0308",
                KeyType::AleoSchnorr => todo!(),
            };

            let invalid_sig: Signature = (config.key_type, HexBinary::from_hex(sig_bytes).unwrap())
                .try_into()
                .unwrap();

            let result =
                validate_session_signature(&session, &signer, &invalid_sig, pub_key, 0, None);

            assert_eq!(
                result.unwrap_err(),
                ContractError::InvalidSignature {
                    session_id: session.id,
                    signer: signer.into(),
                    reason: "Invalid signature".into()
                }
            );
        }
    }

    #[test]
    fn signer_not_a_participant_validation() {
        for config in [ecdsa_setup(), ed25519_setup()] {
            let session = config.session;
            let verifier_set = config.verifier_set;
            let invalid_participant = MockApi::default().addr_make("not_a_participant");

            let result = match verifier_set.signers.get(&invalid_participant.to_string()) {
                Some(signer) => Ok(&signer.pub_key),
                None => Err(ContractError::NotAParticipant {
                    session_id: session.id,
                    signer: invalid_participant.to_string(),
                }),
            };

            assert_eq!(
                result.unwrap_err(),
                ContractError::NotAParticipant {
                    session_id: session.id,
                    signer: invalid_participant.into()
                }
            );
        }
    }
}
