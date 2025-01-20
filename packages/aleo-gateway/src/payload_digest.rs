use aleo_types::address::Address;
use error_stack::Report;
use multisig::verifier_set::{self, VerifierSet};

use crate::{AleoValue, Error};

pub struct PayloadDigest<'a> {
    domain_separator: &'a [u128; 2],
    signer: Address,
    data_hash: String,
}

impl<'a> PayloadDigest<'a> {
    pub fn new(
        domain_separator: &'a [u128; 2],
        verifier_set: &VerifierSet,
        data_hash: String,
    ) -> Result<PayloadDigest<'a>, Report<Error>> {
        let address = verifier_set
            .signers
            .values()
            .next()
            .map(|verifier| match &verifier.pub_key {
                multisig::key::PublicKey::AleoSchnorr(key) => {
                    Ok(Address::try_from(key).map_err(|e| {
                        Report::new(Error::AleoGateway(format!(
                            "Failed to parse address: {}",
                            e
                        )))
                    })?)
                }
                multisig::key::PublicKey::Ecdsa(_) => Err(Report::new(
                    Error::UnsupportedPublicKey("received Ecdsa".to_string()),
                )),
                multisig::key::PublicKey::Ed25519(_) => Err(Report::new(
                    Error::UnsupportedPublicKey("received Ed25519".to_string()),
                )),
            })
            .ok_or_else(|| Report::new(Error::AleoGateway("No signers found".to_string())))??;

        Ok(PayloadDigest {
            domain_separator,
            signer: address,
            data_hash,
        })
    }
}

impl AleoValue for PayloadDigest<'_> {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ domain_separator: [{}], signer: {}, data_hash: {} }}"#,
            self.domain_separator
                .iter()
                .map(|b| format!("{}u128", b))
                .collect::<Vec<_>>()
                .join(", "),
            self.signer,
            self.data_hash
        );

        Ok(res)
    }
}
