use error_stack::Report;
use serde::Deserialize;

use crate::message::Message;
use crate::{AleoValue, Error};

#[derive(Debug, Deserialize)]
pub struct Messages(pub Vec<Message>);

impl From<Vec<Message>> for Messages {
    fn from(v: Vec<Message>) -> Self {
        Messages(v)
    }
}

impl AleoValue for Messages {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = format!(
            r#"{{ messages: [{}] }}"#,
            self.0
                .iter()
                .map(Message::to_aleo_string)
                .collect::<Result<Vec<_>, Report<Error>>>()?
                .join(", ")
        );

        Ok(res)
    }
}
