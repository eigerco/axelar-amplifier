use std::fmt::Display;

use serde::{de, ser};

#[derive(Debug)]
pub enum Error {
    TrailingCharacters,
    F32NotSupported,
    F64NotSupported,
    UnitNotSupported,
    NoneNotSupported,
    UnitStructNotSupported,
    NewtypeVariantNotSupported,
    TupleVariantNotSupported,
    StructVariantNotSupported,
    UnitVariantNotSupported,
    Serialization(String),
    Deserialization(String),
    Eof,
    Syntax,
    BytesNotSupported,
    CharNotSupported,
    OptionNotSupported,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedArrayComma,
    ExpectedMapComma,
    ExpectedMap,
    ExpectedMapEnd,
    EnumNotSupported,
    ExpectedMapColon,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Serialization(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Deserialization(msg.to_string())
    }
}
