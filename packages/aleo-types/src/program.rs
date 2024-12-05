use std::str::FromStr;

use error_stack::{ensure, Report, Result};
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct Program (String);

impl TryFrom<String> for Program {
    type Error = Report<Error>;

    fn try_from(name: String) -> Result<Self, Error> {
        Program::from_str(&name)
    }
}

impl FromStr for Program {
    type Err = Report<Error>;

    fn from_str(name: &str) -> Result<Self, Error> {
        const SUFFIX: &str = ".aleo";

        ensure!(
            name.len() > SUFFIX.len(),
            Error::InvalidProgramName(name.to_string())
        );

        ensure!(
            name.ends_with(SUFFIX),
            Error::InvalidProgramName(name.to_string())
        );

        ensure!(
            name.chars()
                .next()
                .ok_or(Error::InvalidProgramName(name.to_string()))?
                .is_ascii_lowercase(),
            Error::InvalidProgramName(name.to_string())
        );

        ensure!(
            name.chars()
                .skip(1)
                .take(
                    name.len()
                        .saturating_sub(SUFFIX.len().saturating_add(1)),
                )
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            Error::InvalidProgramName(name.to_string())
        );

        Ok(Self (name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use assert_ok::assert_ok;
    use axelar_wasm_std::assert_err_contains;

    use super::*;

    #[test]
    fn validate_aleo_program_name() {
        let program_name = "hello.aleo";
        assert_ok!(Program::from_str(program_name));

        let program_name = "hello123.aleo";
        assert_ok!(Program::from_str(program_name));

        let program_name = "hello_123.aleo";
        assert_ok!(Program::from_str(program_name));
    }

    #[test]
    fn validate_aleo_program_name_errors() {
        let program_name = "hello";
        assert_err_contains!(
            Program::from_str(program_name),
            Error,
            Error::InvalidProgramName(..)
        );

        let program_name = ".aleo";
        assert_err_contains!(
            Program::from_str(program_name),
            Error,
            Error::InvalidProgramName(..)
        );

        let program_name = "";
        assert_err_contains!(
            Program::from_str(program_name),
            Error,
            Error::InvalidProgramName(..)
        );

        let program_name = "hello$.aleo";
        assert_err_contains!(
            Program::from_str(program_name),
            Error,
            Error::InvalidProgramName(..)
        );
    }
}
