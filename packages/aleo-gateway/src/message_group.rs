// Match the representation of a message group in the Aleo gateway.

use error_stack::Report;
use snarkvm_cosmwasm::program::{Network, Zero};
use snarkvm_cosmwasm::types::Group;

use crate::{AleoValue, Error, Message};

/// MN: Number of messages in a message group
/// MG: Number of message groups
#[derive(Debug)]
pub struct MessageGroup<N: Network, const MN: usize = 16, const MG: usize = 3> {
    messages: [[Group<N>; MN]; MG],
}

impl<N: Network, const MN: usize, const MG: usize> MessageGroup<N, MN, MG> {
    pub fn new(messages: Vec<Message>) -> Result<Self, Report<Error>> {
        let max_messages = MN * MG;
        if messages.len() > max_messages {
            todo!()
        }

        let messages = messages
            .iter()
            .map(|m| m.bhp::<N>())
            .collect::<Result<Vec<Group<N>>, Report<Error>>>()?;

        let mut message_groups = [[Group::<N>::zero(); MN]; MG];

        for (i, message) in messages.iter().enumerate() {
            message_groups[i / MN][i % MN] = *message;
        }
        println!("message_groups hash: {:?}", message_groups);

        Ok(Self {
            messages: message_groups,
        })
    }
}

impl<N: Network, const MN: usize, const MG: usize> AleoValue for MessageGroup<N, MN, MG> {
    fn to_aleo_string(&self) -> Result<String, Report<Error>> {
        let res = self
            .messages
            .iter()
            .map(|message_group| {
                format!(
                    r#"[{}]"#,
                    message_group
                        .iter()
                        .map(Group::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        Ok(format!(r#"[{}]"#, res))
    }
}
