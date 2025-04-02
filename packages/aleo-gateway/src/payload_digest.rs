use error_stack::Report;
use multisig::verifier_set::VerifierSet;

use crate::{AleoValue, Error, WeightedSigners};

#[derive(Debug)]
pub struct PayloadDigest<'a> {
    domain_separator: &'a [u128; 2],
    signers: WeightedSigners,
    data_hash: String,
}

impl<'a> PayloadDigest<'a> {
    pub fn new(
        domain_separator: &'a [u128; 2],
        verifier_set: &VerifierSet,
        data_hash: String,
    ) -> Result<PayloadDigest<'a>, Report<Error>> {
        let signers = WeightedSigners::try_from(verifier_set)?;

        Ok(PayloadDigest {
            domain_separator,
            signers,
            data_hash,
        })
    }
}

impl AleoValue for PayloadDigest<'_> {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ domain_separator: [{}], signer: [{}], data_hash: {} }}"#,
            self.domain_separator
                .iter()
                .map(|b| format!("{}u128", b))
                .collect::<Vec<_>>()
                .join(", "),
            self.signers.to_aleo_string()?,
            self.data_hash
        );

        Ok(res)
    }
}
