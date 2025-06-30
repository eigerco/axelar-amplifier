use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("AleoGateway: {0}")]
    AleoGateway(String),
    #[error("InvalidSourceChainLength: expected: {expected}, actual: {actual}")]
    InvalidEncodedStringLength { expected: usize, actual: usize },
    #[error("Invalid ascii character")]
    InvalidAscii,
    #[error("serde_aleo: {0}")]
    SerdeAleo(#[from] serde_aleo::Error),
    #[error("Invalid length: expected {expected}, actual {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("Conversion failed")]
    ConverFailed,
}

pub struct StringEncoder {
    pub buf: Vec<u128>,
}

impl StringEncoder {
    pub fn from_array(aleo_value: &[u128]) -> Self {
        Self {
            buf: aleo_value.to_vec(),
        }
    }

    pub fn encode_string(input: &str) -> Result<Self, Error> {
        Self::encode_bytes(input)
    }

    /// Creates a new StringEncoder from an ASCII string
    pub fn encode_bytes<T: AsRef<[u8]>>(input: T) -> Result<Self, Error> {
        let bytes = input.as_ref();

        // Verify the input is ASCII
        if !bytes.is_ascii() {
            return Err(Error::InvalidAscii);
        }

        // let bytes = input.as_bytes();
        let mut buf = Vec::with_capacity(bytes.len().div_ceil(16));
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

    /// aleo_value expected to be a string of u128 values separated by ", "
    /// example: "1234567890u128, 9876543210u128"
    pub fn from_aleo_value(aleo_value: &str) -> Result<Self, Error> {
        Ok(Self {
            buf: serde_aleo::from_str(aleo_value)?,
        })
    }

    pub fn u128_len(&self) -> usize {
        self.buf.len()
    }

    pub fn consume(self) -> Vec<u128> {
        self.buf
    }

    pub fn to_array<const N: usize>(self) -> Result<[u128; N], Error> {
        if N < self.buf.len() {
            return Err(Error::InvalidLength {
                expected: N,
                actual: self.buf.len(),
            });
        }

        let mut buf = self.buf;
        buf.resize(N, 0);
        buf.try_into().map_err(|_| Error::ConverFailed)
    }

    pub fn as_array_ref<const N: usize>(&self) -> Result<&[u128; N], Error> {
        if N == self.buf.len() {
            return Err(Error::InvalidLength {
                expected: N,
                actual: self.buf.len(),
            });
        }

        self.buf
            .as_slice()
            .try_into()
            .map_err(|_| Error::ConverFailed)
    }

    /// Decodes the StringEncoder, returning the original ASCII string
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
    fn encode_decode() {
        let test_str = "foo";
        let encoded = StringEncoder::encode_string(test_str).unwrap();
        let decoded = encoded.decode();
        assert_eq!(test_str, decoded);
    }

    #[test]
    fn long_string() {
        let test_str = "This is a longer test string that will span multiple u128 values!";
        let encoded = StringEncoder::encode_string(test_str).unwrap();
        let decoded = encoded.decode();
        assert_eq!(test_str, decoded);
    }

    #[test]
    fn empty_string() {
        let test_str = "";
        let encoded = StringEncoder::encode_string(test_str).unwrap();
        let decoded = encoded.decode();
        assert_eq!(test_str, decoded);
    }

    #[test]
    fn from_aleo_value() {
        let aleo_value = "[135867890890980515948416416879465410871u128, 64053233263744786339002611897128269156u128, 135858420114893597535581992180921663488u128]";
        let encoder = StringEncoder::from_aleo_value(aleo_value).unwrap();
        let decoded = encoder.decode();
        assert_eq!(decoded, "f746a117cf5d131700492Bad9f9ba15df5aDa4C4");
    }

    #[test]
    fn adjust_result_size() {
        let encoded =
            StringEncoder::encode_string("f746a117cf5d131700492Bad9f9ba15df5aDa4C4").unwrap();

        assert_eq!(encoded.u128_len(), 3);

        let d: [u128; 3] = encoded.to_array().unwrap();
        assert_eq!(d.len(), 3);

        let encoded =
            StringEncoder::encode_string("f746a117cf5d131700492Bad9f9ba15df5aDa4C4").unwrap();
        let d: [u128; 6] = encoded.to_array().unwrap();
        assert_eq!(d.len(), 6);
    }
}
