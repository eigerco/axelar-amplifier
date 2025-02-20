use std::str::FromStr;
use thiserror::Error;

use snarkvm_cosmwasm::network::Network;
use snarkvm_cosmwasm::program::ToBits;

#[derive(Error, Debug)]
pub enum Error {
    #[error("AleoHash: {0}")]
    Aleo(snarkvm_cosmwasm::program::Error),
}

pub fn bhp_hash<T: AsRef<str>, N: Network>(input: T) -> Result<String, Error> {
    let aleo_value: Vec<bool> = snarkvm_cosmwasm::program::Value::<N>::from_str(input.as_ref())
        .map_err(|e| Error::Aleo(e))?
        .to_bits_le();

    let bits = N::hash_keccak256(&aleo_value).map_err(|e| Error::Aleo(e))?;

    let group = N::hash_to_group_bhp256(&bits).unwrap();

    Ok(group.to_string())
}

pub fn keccak256<T: AsRef<str>, N: Network>(input: T) -> Result<[u8; 32], Error> {
    let aleo_value: Vec<bool> = snarkvm_cosmwasm::program::Value::<N>::from_str(input.as_ref())
        .map_err(|e| Error::Aleo(e))?
        .to_bits_le();

    let bits = N::hash_keccak256(&aleo_value).map_err(|e| Error::Aleo(e))?;

    let mut hash = [0u8; 32];
    for (i, b) in bits.chunks(8).enumerate() {
        let mut byte = 0u8;
        for (i, bit) in b.iter().enumerate() {
            if *bit {
                byte |= 1 << i;
            }
        }
        hash[i] = byte;
    }

    Ok(hash)
}
