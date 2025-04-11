use error_stack::Report;
use multisig::verifier_set::VerifierSet;
use snarkvm_cosmwasm::program::Network;

use crate::proof::Proof;
use crate::{AleoValue, Error, MessageGroup, WeightedSigners};

pub struct ExecuteDataMessages <N: Network, const MN: usize = 16, const MG: usize = 3> {
    proof: Proof,
    payload: MessageGroup<N, MN, MG>,
}

impl<N: Network, const MN: usize, const MG: usize> ExecuteDataMessages<N, MN, MG> {
    pub fn new(proof: Proof, payload: MessageGroup<N, MN, MG>) -> Self {
        ExecuteDataMessages { proof, payload }
    }
}

impl<N: Network, const MN: usize, const MG: usize> AleoValue for ExecuteDataMessages<N, MN, MG> {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ proof: {}, message: {} }}"#,
            self.proof.to_aleo_string()?,
            self.payload.to_aleo_string()?
        );

        Ok(res)
    }
}

pub struct ExecuteDataVerifierSet<const GROUP_SIZE: usize = 2, const GROUPS: usize = 2> {
    proof: Proof,
    payload: WeightedSigners<GROUP_SIZE, GROUPS>,
}

impl<const GROUP_SIZE: usize, const GROUPS: usize> ExecuteDataVerifierSet<GROUP_SIZE, GROUPS> {
    pub fn new(proof: Proof, payload: VerifierSet) -> ExecuteDataVerifierSet<GROUP_SIZE, GROUPS> {
        let payload = WeightedSigners::<GROUP_SIZE, GROUPS>::try_from(&payload).unwrap();
        ExecuteDataVerifierSet { proof, payload }
    }
}

impl AleoValue for ExecuteDataVerifierSet {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ proof: {}, payload: {} }}"#,
            self.proof.to_aleo_string()?,
            self.payload.to_aleo_string()?
        );

        Ok(res)
    }
}
