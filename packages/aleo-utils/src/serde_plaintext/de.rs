use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{forward_to_deserialize_any, Deserialize};

use super::error::Error;
use super::parser::{
    parse_aleo_literal, parse_bool, parse_identifier, parse_numeric_literal, parse_whitespace,
};

pub struct PlaintextDeserializer<'de> {
    input: &'de str,
}

impl<'de> PlaintextDeserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Self { input }
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T, Error>
where
    T: Deserialize<'a>,
{
    let mut deserializer = PlaintextDeserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

impl<'de> PlaintextDeserializer<'de> {
    fn peek_char(&mut self) -> Result<char, Error> {
        self.input.chars().next().ok_or(Error::Eof)
    }

    fn next_char(&mut self) -> Result<char, Error> {
        let ch = self.peek_char()?;
        self.input = &self.input[ch.len_utf8()..];
        Ok(ch)
    }

    fn consume_whitespace(&mut self) {
        if let Ok((rest, _result)) = parse_whitespace(self.input) {
            self.input = rest;
        }
    }
}

impl<'de> de::Deserializer<'de> for &'_ mut PlaintextDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        if let Ok((rest, result)) = parse_bool(self.input) {
            self.input = rest;
            let result = result.parse::<bool>().unwrap();
            visitor.visit_bool(result)
        } else if let Ok((rest, result)) = parse_identifier(self.input) {
            self.input = rest;
            visitor.visit_borrowed_str(result)
        } else if let Ok((rest, (digits, suffix))) = parse_numeric_literal(self.input) {
            use super::parser::NumericSuffix::*;
            self.input = rest;
            match suffix {
                U8 => visitor.visit_u8(digits.parse::<u8>().unwrap()),
                U16 => visitor.visit_u16(digits.parse::<u16>().unwrap()),
                U32 => visitor.visit_u32(digits.parse::<u32>().unwrap()),
                U64 => visitor.visit_u64(digits.parse::<u64>().unwrap()),
                U128 => visitor.visit_u128(digits.parse::<u128>().unwrap()),
                I8 => visitor.visit_i8(digits.parse::<i8>().unwrap()),
                I16 => visitor.visit_i16(digits.parse::<i16>().unwrap()),
                I32 => visitor.visit_i32(digits.parse::<i32>().unwrap()),
                I64 => visitor.visit_i64(digits.parse::<i64>().unwrap()),
                I128 => visitor.visit_i128(digits.parse::<i128>().unwrap()),
            }
        } else if let Ok((rest, result)) = parse_aleo_literal(self.input) {
            self.input = rest;
            visitor.visit_borrowed_str(result)
        } else {
            Err(Error::Syntax)
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 str string identifier
        ignored_any
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::F32NotSupported)
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::F64NotSupported)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::CharNotSupported)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::BytesNotSupported)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::BytesNotSupported)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::OptionNotSupported)
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnitNotSupported)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnitStructNotSupported)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_whitespace();
        if self.next_char()? == '[' {
            self.consume_whitespace();

            let value = visitor.visit_seq(CommaSeparated::new(self))?;

            self.consume_whitespace();
            if self.next_char()? == ']' {
                self.consume_whitespace();
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_whitespace();
        if self.next_char()? == '{' {
            self.consume_whitespace();

            let value = visitor.visit_map(CommaSeparated::new(self))?;

            self.consume_whitespace();
            if self.next_char()? == '}' {
                self.consume_whitespace();

                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::EnumNotSupported)
    }
}

struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut PlaintextDeserializer<'de>,
    first: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(de: &'a mut PlaintextDeserializer<'de>) -> Self {
        CommaSeparated { de, first: true }
    }
}

impl<'a, 'de> SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.consume_whitespace();
        if self.de.peek_char()? == ']' {
            return Ok(None);
        }

        if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedArrayComma);
        }

        self.first = false;

        self.de.consume_whitespace();
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'a, 'de> MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        self.de.consume_whitespace();
        if self.de.peek_char()? == '}' {
            return Ok(None);
        }

        if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedMapComma);
        }

        self.first = false;

        self.de.consume_whitespace();
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.consume_whitespace();
        if self.de.next_char()? != ':' {
            return Err(Error::ExpectedMapColon);
        }

        self.de.consume_whitespace();
        seed.deserialize(&mut *self.de)
    }
}
