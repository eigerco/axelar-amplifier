use std::convert::TryFrom;

use error_stack::Report;

use crate::{AleoValue, Error};

pub struct StringEncoder {
    pub buf: Vec<u128>,
}

impl StringEncoder {
    /// Creates a new StringEncoder from an ASCII string
    pub fn encode_string(input: &str) -> Result<Self, Report<Error>> {
        // Verify the input is ASCII
        if !input.is_ascii() {
            return Err(Report::new(Error::InvalidAscii));
        }

        let bytes = input.as_bytes();
        let mut buf = Vec::with_capacity((bytes.len() + 15) / 16);
        let mut current_value: u128 = 0;
        let mut position = 0;

        for &byte in bytes {
            // Shift left by 8 and add new byte
            current_value = (current_value << 8) | u128::from(byte);
            position += 1;

            // When we have 16 bytes, push to result
            if position == 16 {
                buf.push(current_value);
                current_value = 0;
                position = 0;
            }
        }

        // Handle any remaining bytes
        if position > 0 {
            // Left shift remaining bytes to maintain consistency
            current_value <<= 8 * (16 - position);
            buf.push(current_value);
        }

        Ok(Self { buf })
    }

    // aleo_value expected to be a string of u128 values separated by ", "
    // example: "1234567890u128, 9876543210u128"
    pub fn from_aleo_value(aleo_value: &str) -> Result<Self, Report<Error>> {
        Ok(Self {
            buf: aleo_value
                .split(", ")
                .map(|s| s.replace("u128", "").parse::<u128>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| Error::AleoGateway(format!("Failed to parse u128: {e:?}")))?,
        })
    }

    pub fn u128_len(&self) -> usize {
        self.buf.len()
    }

    pub fn consume(self) -> Vec<u128> {
        self.buf
    }

    /// Decodes and consumes the StringEncoder, returning the original ASCII string
    pub fn decode(&self) -> String {
        let mut result = Vec::new();

        for (i, value) in self.buf.iter().enumerate() {
            // Extract all possible bytes from the u128
            for j in 0..16 {
                let shift = 8 * (15 - j);
                let byte = ((value >> shift) & 0xFF) as u8;
                // Only add non-zero bytes from the last chunk
                if i < self.buf.len() - 1 || byte != 0 {
                    result.push(byte);
                }
            }
        }

        // Trim any trailing zeros
        while result.last() == Some(&0) {
            result.pop();
        }

        // Safe to unwrap as we verified ASCII in new()
        String::from_utf8(result).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let test_str = "foo";
        let encoded = StringEncoder::encode_string(test_str).unwrap();
        let decoded = encoded.decode();
        assert_eq!(test_str, decoded);
    }

    #[test]
    fn test_long_string() {
        let test_str = "This is a longer test string that will span multiple u128 values!";
        let encoded = StringEncoder::encode_string(test_str).unwrap();
        let decoded = encoded.decode();
        assert_eq!(test_str, decoded);
    }

    #[test]
    fn test_empty_string() {
        let test_str = "";
        let encoded = StringEncoder::encode_string(test_str).unwrap();
        let decoded = encoded.decode();
        assert_eq!(test_str, decoded);
    }
}
