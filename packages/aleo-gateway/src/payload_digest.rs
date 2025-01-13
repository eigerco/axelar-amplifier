use error_stack::Report;

use crate::{AleoValue, Error};

pub struct PayloadDigest<'a> {
    domain_separator: &'a [u8; 32],
    signers_hash: &'a [u8; 32],
    data_hash: &'a [u8; 32],
}

impl<'a> PayloadDigest<'a> {
    pub fn new(
        domain_separator: &'a [u8; 32],
        signers_hash: &'a [u8; 32],
        data_hash: &'a [u8; 32],
    ) -> PayloadDigest<'a> {
        PayloadDigest {
            domain_separator,
            signers_hash,
            data_hash,
        }
    }
}

impl AleoValue for PayloadDigest<'_> {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ domain_separator: [ {} ], signers_hash: [ {} ], data_hash: [ {} ] }}"#,
            self.domain_separator
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", "),
            self.signers_hash
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", "),
            self.data_hash
                .iter()
                .map(|b| format!("{}u8", b))
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(res)
    }
}
