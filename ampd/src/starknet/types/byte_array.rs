use core::fmt;

use itertools::Itertools;
use starknet_core::types::{FieldElement, ValueOutOfRangeError};
use starknet_core::utils::parse_cairo_short_string;
use thiserror::Error;

#[derive(Debug)]
pub struct ByteArray {
    /// The data byte array. Contains 31-byte chunks of the byte array.
    data: Vec<FieldElement>,
    /// The bytes that remain after filling the data array with full 31-byte
    /// chunks
    pending_word: FieldElement,
    /// The byte count of the pending_word
    pending_word_length: u8, // can't be more than 30 bytes
}

impl Default for ByteArray {
    fn default() -> Self {
        Self {
            data: Default::default(),
            pending_word: Default::default(),
            pending_word_length: Default::default(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ByteArrayError {
    #[error("Invalid byte array - {0}")]
    InvalidByteArray(String),
    #[error("Failed to parse felt - {0}")]
    ParsingFelt(#[from] ValueOutOfRangeError),
}

/// The Vec<FieldElement> should be only the representation of the ByteArray
/// type as described in this document:
/// https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_of_byte_arrays
impl TryFrom<Vec<FieldElement>> for ByteArray {
    type Error = ByteArrayError;

    fn try_from(data: Vec<FieldElement>) -> Result<Self, Self::Error> {
        let mut byte_array = ByteArray {
            ..Default::default()
        };

        if data.len() < 3 {
            return Err(ByteArrayError::InvalidByteArray(
                "vec should have minimum 3 elements".to_owned(),
            ));
        }

        // word count is always the first element
        let word_count: u32 = match data[0].try_into() {
            Ok(wc) => wc,
            Err(err) => return Err(ByteArrayError::ParsingFelt(err)),
        };

        // vec element count should be whatever the word count is + 3
        // the 3 stands for the minimum 3 elements:
        // - word count
        // - pending_word
        // - pendint_word_length
        let is_arr_el_count_valid = usize::try_from(word_count + 3)
            .map(|count| count == data.len())
            .unwrap_or(false);
        if !is_arr_el_count_valid {
            return Err(ByteArrayError::InvalidByteArray(
                "invalid byte array structure".to_owned(),
            ));
        }

        // pending word byte count is always the last element
        let pending_word_length: u8 = match data[data.len() - 1].try_into() {
            Ok(bc) => bc,
            Err(err) => return Err(ByteArrayError::ParsingFelt(err)),
        };
        byte_array.pending_word_length = pending_word_length;

        // pending word is always the next to last element
        let pending_word = data[data.len() - 2];
        byte_array.pending_word = pending_word;

        if word_count > 0 {
            byte_array.data = data[1..data.len() - 2].to_vec();
        }

        Ok(byte_array)

        // TODO:
        // - If word count is 0 - convert the pending word to a string
        // - If word count > 0:
        //   - for i=2; i < 2+eventData[1]; i++
        //     - cut all leading 0s
        //     - concatenate all field element hex bytes resulting in
        //       31_word_bytes
        //     - parse felt 1 to u32 and take element parsedFelt+2 which is the
        //       pending_word
        //       - parse elelemtn parsedFelt+3 as u8, which is
        //         pending_word_bytes_length
        //       - take pending_words_byte_length worth of bytes from the
        //         pending_word
        //     - take the pending_word bytes and concatenate them with the
        //       previous 31_word_bytes
        //     - Convert those bytes to a string
    }
}

// TODO: Write tests
impl fmt::Display for ByteArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let felts_making_a_string: Vec<FieldElement> =
            vec![self.data.clone(), vec![self.pending_word]].concat();
        let as_string_result: Result<String, _> = felts_making_a_string
            .into_iter()
            .map(|felt| parse_cairo_short_string(&felt))
            .collect();

        let as_string = match as_string_result {
            Ok(s) => s,
            Err(_) => return Err(fmt::Error),
        };

        write!(f, "{}", as_string)
    }
}

#[cfg(test)]
mod byte_array_tests {
    use std::str::FromStr;

    use starknet_core::types::FieldElement;

    use crate::starknet::types::byte_array::ByteArray;

    #[test]
    fn byte_array_single_pending_word_to_string_valid() {
        // Example for a small string (fits in a single felt) taken from here:
        // https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_of_byte_arrays
        //
        // So this is the string "hello"
        let data = vec![
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000005",
            )
            .unwrap(),
        ];

        let byte_array = ByteArray::try_from(data).unwrap();
        assert_eq!("hello", byte_array.to_string());
    }

    #[test]
    fn byte_array_to_long_string_valid() {
        // Example for a long string (doesn't fit in a single felt) taken from here:
        // https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_of_byte_arrays
        //
        // So this is the string "Long long string, a lot more than 31 characters that
        // wouldn't even fit in two felts, so we'll have at least two felts and a
        // pending word."
        let data = vec![
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000004",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00004c6f6e67206c6f6e6720737472696e672c2061206c6f74206d6f72652074",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x000068616e2033312063686172616374657273207468617420776f756c646e27",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x000074206576656e2066697420696e2074776f2066656c74732c20736f207765",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000276c6c2068617665206174206c656173742074776f2066656c747320616e",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000006420612070656e64696e6720776f72642e",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000011",
            )
            .unwrap(),
        ];

        let byte_array = ByteArray::try_from(data).unwrap();
        assert_eq!("Long long string, a lot more than 31 characters that wouldn't even fit in two felts, so we'll have at least two felts and a pending word.", byte_array.to_string());
    }

    #[test]
    fn try_from_vec_count_less_then_3() {
        let data = vec![FieldElement::from_str(
            "0x0000000000000000000000000000000000000000000000000000000000000005",
        )
        .unwrap()];

        let byte_array_err = ByteArray::try_from(data);
        assert!(byte_array_err.is_err());
    }

    #[test]
    fn try_from_non_u32_word_count() {
        let data = vec![
            // should be 0, because the message is short
            // enough to fit in a single FieldElement
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000005",
            )
            .unwrap(),
        ];

        let byte_array_err = ByteArray::try_from(data);
        assert!(byte_array_err.is_err());
    }
    #[test]
    fn try_from_invalid_byte_array_element_count() {
        let data = vec![
            // should be 0, because the message is short
            // enough to fit in a single FieldElement
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000005",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000005",
            )
            .unwrap(),
        ];

        let byte_array_err = ByteArray::try_from(data);
        assert!(byte_array_err.is_err());
    }

    #[test]
    fn try_from_non_u8_pending_word_length() {
        // Example for a small string (fits in a single felt) taken from here:
        // https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_of_byte_arrays
        //
        // So this is the string "hello"
        let data = vec![
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap(),
        ];

        let byte_array = ByteArray::try_from(data);
        assert!(byte_array.is_err());
    }

    #[test]
    fn try_from_valid_only_pending_word() {
        // Example for a small string (fits in a single felt) taken from here:
        // https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_of_byte_arrays
        //
        // So this is the string "hello"
        let data = vec![
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000005",
            )
            .unwrap(),
        ];

        let byte_array = ByteArray::try_from(data).unwrap();

        assert_eq!(byte_array.data, vec![]);
        assert_eq!(
            byte_array.pending_word,
            FieldElement::from_str(
                "0x00000000000000000000000000000000000000000000000000000068656c6c6f",
            )
            .unwrap()
        );
        assert_eq!(byte_array.pending_word_length, 5);
    }

    #[test]
    fn try_from_valid_one_big_string_split_in_multiple_data_elements() {
        // Example for a long string (doesn't fit in a single felt) taken from here:
        // https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_of_byte_arrays
        //
        // So this is the string "Long long string, a lot more than 31 characters that
        // wouldn't even fit in two felts, so we'll have at least two felts and a
        // pending word."
        let data = vec![
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000004",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x00004c6f6e67206c6f6e6720737472696e672c2061206c6f74206d6f72652074",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x000068616e2033312063686172616374657273207468617420776f756c646e27",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x000074206576656e2066697420696e2074776f2066656c74732c20736f207765",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000276c6c2068617665206174206c656173742074776f2066656c747320616e",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000006420612070656e64696e6720776f72642e",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000011",
            )
            .unwrap(),
        ];

        let byte_array = ByteArray::try_from(data).unwrap();

        assert_eq!(
            byte_array.data,
            vec![
                FieldElement::from_str(
                    "0x00004c6f6e67206c6f6e6720737472696e672c2061206c6f74206d6f72652074",
                )
                .unwrap(),
                FieldElement::from_str(
                    "0x000068616e2033312063686172616374657273207468617420776f756c646e27",
                )
                .unwrap(),
                FieldElement::from_str(
                    "0x000074206576656e2066697420696e2074776f2066656c74732c20736f207765",
                )
                .unwrap(),
                FieldElement::from_str(
                    "0x0000276c6c2068617665206174206c656173742074776f2066656c747320616e",
                )
                .unwrap()
            ]
        );
        assert_eq!(
            byte_array.pending_word,
            FieldElement::from_str(
                "0x0000000000000000000000000000006420612070656e64696e6720776f72642e",
            )
            .unwrap()
        );
        assert_eq!(byte_array.pending_word_length, 17);
    }

    #[test]
    fn try_from_valid_one_very_big_string() {
        // Example for a long string (doesn't fit in a single felt) taken from here:
        // https://docs.starknet.io/documentation/architecture_and_concepts/Smart_Contracts/serialization_of_Cairo_types/#serialization_of_byte_arrays
        //
        // So this is the string "Long string, more than 31 characters."
        let data = vec![
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x004c6f6e6720737472696e672c206d6f7265207468616e203331206368617261",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x000000000000000000000000000000000000000000000000000063746572732e",
            )
            .unwrap(),
            FieldElement::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000006",
            )
            .unwrap(),
        ];

        let byte_array = ByteArray::try_from(data).unwrap();

        assert_eq!(
            byte_array.data,
            vec![FieldElement::from_str(
                "0x004c6f6e6720737472696e672c206d6f7265207468616e203331206368617261",
            )
            .unwrap()]
        );
        assert_eq!(
            byte_array.pending_word,
            FieldElement::from_str(
                "0x000000000000000000000000000000000000000000000000000063746572732e",
            )
            .unwrap()
        );
        assert_eq!(byte_array.pending_word_length, 6);
    }
}
