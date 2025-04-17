// Match the representation of a message group in the Aleo gateway.

use error_stack::{ensure, Report};
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
        let max_messages = MN.saturating_mul(MG);
        ensure!(
            messages.len() <= max_messages,
            Error::InvalidMessageGroupLength {
                max: max_messages,
                actual: messages.len(),
            }
        );

        let mut message_groups = [[Group::<N>::zero(); MN]; MG];
        for (i, message) in messages.iter().enumerate() {
            let row = i.checked_div(MN).ok_or(Error::CheckedDivision(i, MN))?;
            let col = i.checked_rem(MN).ok_or(Error::CheckedRemainder(i, MN))?;
            message_groups[row][col] = message.bhp::<N>()?;
        }

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
