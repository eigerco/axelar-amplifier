use crate::{
    key::{KeyType, PublicKey},
    multisig::Multisig,
    state::{load_pub_key, load_session_signatures},
    verifier_set::VerifierSet,
};

use super::*;

pub fn get_multisig(deps: Deps, session_id: Uint64) -> StdResult<Multisig> {
    let session = SIGNING_SESSIONS.load(deps.storage, session_id.into())?;

    let verifier_set = VERIFIER_SETS.load(deps.storage, &session.verifier_set_id)?;
    let signatures = load_session_signatures(deps.storage, session.id.u64())?;

    Ok(Multisig {
        state: session.state,
        verifier_set,
        signatures,
    })
}

pub fn get_verifier_set(deps: Deps, verifier_set_id: String) -> StdResult<VerifierSet> {
    VERIFIER_SETS.load(deps.storage, &verifier_set_id)
}

pub fn get_public_key(deps: Deps, verifier: Addr, key_type: KeyType) -> StdResult<PublicKey> {
    let raw = load_pub_key(deps.storage, verifier, key_type)?;
    Ok(PublicKey::try_from((key_type, raw)).expect("could not decode pub key"))
}
