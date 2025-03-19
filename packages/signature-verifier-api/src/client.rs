use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_std::{
    to_json_binary, Addr, HexBinary, QuerierWrapper, QueryRequest, Uint64, WasmQuery,
};
use error_stack::{Report, Result};

use crate::msg::QueryMsg;

pub struct SignatureVerifier<'a> {
    pub address: Addr,
    pub querier: QuerierWrapper<'a>,
}

impl SignatureVerifier<'_> {
    fn query<U: DeserializeOwned + 'static>(&self, msg: &QueryMsg) -> Result<U, Error> {
        self.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.address.to_string(),
                msg: to_json_binary(msg).expect("msg should always be serializable"),
            }))
            .map_err(|e| {
                Report::new(Error::QuerySignatureVerifier(
                    self.address.to_string(),
                    e.to_string(),
                ))
            })
    }

    pub fn verify_signature(
        &self,
        signature: HexBinary,
        message: HexBinary,
        public_key: HexBinary,
        signer_address: String,
        session_id: Uint64,
    ) -> Result<bool, Error> {
        self.query(&QueryMsg::VerifySignature {
            signature,
            message,
            public_key,
            signer_address,
            session_id,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not query the signature verifier contract, address: '{0}', '{1}'")]
    QuerySignatureVerifier(String, String),
}
