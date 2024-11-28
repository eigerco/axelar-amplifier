use starknet_core::types::Felt;
use starknet_types::address::Address;
use starknet_types::weighted_signer::Signer;

use super::errors::Parse;
// use super::StarknetEventsPage;

/// Wrapper for `EventsPage` to implement `TryFrom` trait
pub struct StarknetEventsPage(pub starknet_core::types::EventsPage);

/// Represents a set of weighted signers
/// TODO: reason why we need this struct, can't we just import from
/// packages/evm_gateway crate?
#[derive(Debug, Clone)]
pub struct WeightedSigners {
    pub signers: Vec<Signer>,
    pub threshold: u128,
    pub nonce: [u8; 32],
}

/// Represents a Starknet SignersRotated event.
#[derive(Debug, Clone)]
pub struct SignersRotated {
    /// The transaction hash
    pub tx_signature: Felt,
    /// The epoch number when this rotation occurred
    pub epoch: u64,
    /// The hash of the new signers
    pub signers_hash: [u8; 32],
    /// The new set of weighted signers with their voting power
    pub signers: WeightedSigners,
}

/// Converts a `StarknetEventsPage` into a vector of `SignersRotated` events.
///
/// The event data format is:
/// [num_signers, signer1, weight1, signer2, weight2, ..., threshold, nonce]
///
/// # Errors
///
/// Returns a `Parse` error if:
/// - The events page is empty
/// - Required event keys (epoch, signers hash) are missing
/// - Event data format is invalid or incomplete
/// - Numeric conversions fail
impl TryFrom<StarknetEventsPage> for Vec<SignersRotated> {
    type Error = Parse;

    fn try_from(events: StarknetEventsPage) -> Result<Self, Self::Error> {
        // No events no conversion
        if events.0.events.is_empty() {
            return Err(Parse::NoEventsToProcess);
        }

        let mut buf = Self::with_capacity(events.0.events.len());

        for event in &events.0.events {
            if event.data.is_empty() {
                continue;
            }
            if event.keys.is_empty() {
                continue;
            }

            let tx_hash = event.transaction_hash;
            // To avoid multiple convertions
            let tx_hash_str = tx_hash.to_string();

            // it starts at 2 because 0 is the selector and 1 is the from_address
            let epoch_index = 2;
            // INFO: there might be better way to convert to u64
            let epoch = event
                .keys
                .get(epoch_index)
                .ok_or(Parse::IncorrectEpoch(tx_hash_str.clone()))?
                .to_string()
                .parse::<u64>()
                .map_err(|_| Parse::IncorrectEpoch(tx_hash_str.clone()))?;

            // Construct signers hash
            let mut signers_hash = [0_u8; 32];
            let lsb = event
                .keys
                .get(epoch_index + 1)
                .map(Felt::to_bytes_be)
                .ok_or_else(|| Parse::MissingSignersHash(tx_hash_str.clone()))?;
            let msb = event
                .keys
                .get(epoch_index + 2)
                .map(Felt::to_bytes_be)
                .ok_or_else(|| Parse::MissingSignersHash(tx_hash_str.clone()))?;
            signers_hash[..16].copy_from_slice(&msb[16..]);
            signers_hash[16..].copy_from_slice(&lsb[16..]);

            // Parse signers array from event data
            let mut buff_signers = vec![];

            let signers_index = 0;
            let signers_len = event.data[signers_index]
                .to_string()
                .parse::<usize>()
                .map_err(|_| {
                    Parse::FailedToParsePayloadData(
                        tx_hash_str.clone(),
                        "failed to parse signers length".to_string(),
                    )
                })?;
            let signers_end_index = signers_index + signers_len * 2;

            // Parse signers and weights
            for i in 0..signers_len {
                let signer_index = signers_index + 1 + (i * 2);
                let weight_index = signer_index + 1;

                // Get signer address as bytes
                let signer_bytes = event.data[signer_index].to_bytes_be();

                // Create Address from bytes, skipping first 12 bytes since Address is 20 bytes
                let signer = Address::try_from(&signer_bytes[12..]).map_err(|_| {
                    Parse::FailedToParsePayloadData(
                        tx_hash_str.clone(),
                        "failed to parse signer address".to_string(),
                    )
                })?;

                // Parse weight
                let weight = event.data[weight_index]
                    .to_string()
                    .parse::<u128>()
                    .map_err(|_| {
                        Parse::FailedToParsePayloadData(
                            tx_hash_str.clone(),
                            "failed to parse signer weight".to_string(),
                        )
                    })?;

                buff_signers.push(Signer::new(signer, weight));
            }

            // Parse threshold
            let threshold = event
                .data
                .get(signers_end_index)
                .ok_or_else(|| Parse::IncorrectThreshold(tx_hash_str.clone()))?
                .to_string()
                .parse::<u128>()
                .map_err(|_| Parse::IncorrectThreshold(tx_hash_str.clone()))?;

            // Parse nonce
            let mut nonce = [0_u8; 32];
            let lsb = event
                .keys
                .get(signers_end_index + 1)
                .map(Felt::to_bytes_be)
                .ok_or_else(|| Parse::MissingNonce(tx_hash_str.clone()))?;
            let msb = event
                .keys
                .get(signers_end_index + 2)
                .map(Felt::to_bytes_be)
                .ok_or_else(|| Parse::MissingNonce(tx_hash_str.clone()))?;
            nonce[..16].copy_from_slice(&msb[16..]);
            nonce[16..].copy_from_slice(&lsb[16..]);

            buf.push(SignersRotated {
                tx_signature: tx_hash,
                epoch,
                signers_hash,
                signers: WeightedSigners {
                    signers: buff_signers,
                    threshold,
                    nonce,
                },
            });
        }

        if buf.is_empty() {
            Err(Parse::NoEventsToProcess)
        } else {
            Ok(buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::stream::{FuturesUnordered, StreamExt};
    use starknet_core::types::{EmittedEvent, EventsPage, Felt};

    use super::*;

    async fn get_valid_event() -> (Vec<Felt>, Vec<Felt>, Felt, Felt) {
        let keys_data: Vec<Felt> = vec![
            Felt::from_hex("0x01815547484542c49542242a23bc0a1b762af99232f38c0417050825aea8fc93")
                .unwrap(),
            Felt::from_hex("0x0268929df65ee595bb8592323f981351efdc467d564effc6d2e54d2e666e43ca")
                .unwrap(),
            Felt::from_hex("0x01").unwrap(),
            Felt::from_hex("0xd4203fe143363253c89a27a26a6cb81f").unwrap(),
            Felt::from_hex("0xe23e7704d24f646e5e362c61407a69d2").unwrap(),
        ];

        let event_data: Vec<Felt> = vec![
            Felt::from_hex("0x01").unwrap(),
            Felt::from_hex("0x3ec7d572a0fe479768ac46355651f22a982b99cc").unwrap(),
            Felt::from_hex("0x01").unwrap(),
            Felt::from_hex("0x01").unwrap(),
            Felt::from_hex("0x2fe49d").unwrap(),
            Felt::from_hex("0x00").unwrap(),
        ];
        (
            keys_data,
            event_data,
            // sender_address
            Felt::from_hex("0x0282b4492e08d8b6bbec8dfe7412e42e897eef9c080c5b97be1537433e583bdc")
                .unwrap(),
            // tx_hash
            Felt::from_hex("0x04663231715b17dd58cd08e63d6b31d2c86b158d4730da9a1b75ca2452c9910c")
                .unwrap(),
        )
    }

    /// Generate a set of data with random modifications
    async fn get_malformed_event() -> (Vec<Felt>, Vec<Felt>, Felt, Felt) {
        let (mut keys_data, mut event_data, sender_address, tx_hash) = get_valid_event().await;
        // Randomly remove an element from either vector
        match rand::random::<bool>() {
            true if !keys_data.is_empty() => {
                let random_index = rand::random::<usize>() % keys_data.len();
                keys_data.remove(random_index);
            }
            false if !event_data.is_empty() => {
                let random_index = rand::random::<usize>() % event_data.len();
                event_data.remove(random_index);
            }
            _ => {}
        }

        // Randomly corrupt data values
        if rand::random::<bool>() {
            if let Some(elem) = keys_data.first_mut() {
                *elem = Felt::from_hex_be("0xdeadbeef").unwrap();
            }
        }
        if rand::random::<bool>() {
            if let Some(elem) = event_data.first_mut() {
                *elem = Felt::from_hex_be("0xcafebabe").unwrap();
            }
        }

        (keys_data, event_data, sender_address, tx_hash)
    }

    #[tokio::test]
    async fn test_try_from_events_page_happy_scenario() {
        let (keys_data, event_data, sender_address, tx_hash) = get_valid_event().await;

        let event = EmittedEvent {
            data: event_data,
            from_address: sender_address,
            keys: keys_data,
            transaction_hash: tx_hash,
            block_hash: None,
            block_number: None,
        };
        let events_page = StarknetEventsPage(EventsPage {
            events: vec![event],
            continuation_token: None,
        });
        let result = Vec::<SignersRotated>::try_from(events_page);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_try_from_events_page_happy_scenario_multiple_events() {
        let (keys_data, event_data, sender_address, tx_hash) = get_valid_event().await;

        let event = EmittedEvent {
            data: event_data,
            from_address: sender_address,
            keys: keys_data,
            transaction_hash: tx_hash,
            block_hash: None,
            block_number: None,
        };
        let events_page = StarknetEventsPage(EventsPage {
            events: vec![event.clone(), event],
            continuation_token: None,
        });
        let result = Vec::<SignersRotated>::try_from(events_page);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_try_from_empty_events_page() {
        let events_page = StarknetEventsPage(EventsPage {
            events: vec![],
            continuation_token: None,
        });
        let result = Vec::<SignersRotated>::try_from(events_page);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            Parse::NoEventsToProcess.to_string()
        );
    }

    #[tokio::test]
    async fn test_try_from_events_page_missing_data() {
        let (keys_data, _, sender_address, tx_hash) = get_valid_event().await;

        let event = EmittedEvent {
            data: vec![],
            from_address: sender_address,
            keys: keys_data,
            transaction_hash: tx_hash,
            block_hash: None,
            block_number: None,
        };
        let events_page = StarknetEventsPage(EventsPage {
            events: vec![event],
            continuation_token: None,
        });
        let result = Vec::<SignersRotated>::try_from(events_page);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_from_events_page_missing_keys() {
        let (_, event_data, sender_address, tx_hash) = get_valid_event().await;

        let event = EmittedEvent {
            data: event_data,
            from_address: sender_address,
            keys: vec![],
            transaction_hash: tx_hash,
            block_hash: None,
            block_number: None,
        };
        let events_page = StarknetEventsPage(EventsPage {
            events: vec![event],
            continuation_token: None,
        });
        let result = Vec::<SignersRotated>::try_from(events_page);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_from_events_page_randomly_malformed_data_x1000() {
        let mut futures = FuturesUnordered::new();

        for _ in 0..1000 {
            futures.push(async {
                let (_, event_data, sender_address, tx_hash) = get_malformed_event().await;
                let event = EmittedEvent {
                    data: event_data,
                    from_address: sender_address,
                    keys: vec![],
                    transaction_hash: tx_hash,
                    block_hash: None,
                    block_number: None,
                };
                let events_page = StarknetEventsPage(EventsPage {
                    events: vec![event],
                    continuation_token: None,
                });
                Vec::<SignersRotated>::try_from(events_page).is_err()
            });
        }

        // if any conversion succeeded then it should have failed
        while let Some(result) = futures.next().await {
            if !result {
                panic!("expected conversion to fail for malformed event");
            }
        }
    }
}
