use error_stack::Report;

use crate::{AleoValue, Error};

/*
sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml
*/

// TODO: nonce is skipped
#[derive(Clone, Debug)]
pub struct RawSignature {
    pub signature: Vec<u8>,
}

impl RawSignature {
    pub fn new(sig: &str) -> Self {
        let signature = sig.as_bytes().to_vec();
        RawSignature { signature }
    }
}

impl Default for RawSignature {
    fn default() -> Self {
        RawSignature::new("sign1tg9nwza05k89vyspuemc8r5vkz54f3v3gl4s6x05atqtgwnyyvq8ccamwql5txztuevplttghu5eyprlmth6ysgvsz9mgzjslf8guq5r22qjwn4zc0pzv87twjygsz9m7ekljmuw4jpzf68rwuq99r0tp735vs6220q7tp60nr7llkwstcvu49wdhydx5x2s3sftjskzawhqvnvcgml")
    }
}

impl AleoValue for RawSignature {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = String::from_utf8(self.signature.clone()).map_err(|e| {
            Report::new(Error::AleoGateway(format!(
                "Failed to convert to utf8: {}",
                e
            )))
        })?;
        Ok(res)
    }
}
