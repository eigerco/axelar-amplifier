use axelar_wasm_std::hash::Hash;
use cosmwasm_std::HexBinary;
use error_stack::{Result, ResultExt};
use multisig::msg::SignerWithSig;
use multisig::verifier_set::VerifierSet;
use starknet_message::{CairoSerialize, StarknetMessage};
use starknet_types_core::felt::Felt;

use crate::encoding::starknet_types::{
    data_hash_poseidon, payload_digest_poseidon, signer_hash_poseidon, StarkSignature,
    StarknetCommandType, StarknetProof, StarknetWeightedSigners,
};
use crate::error::ContractError;
use crate::payload::Payload;

/// Convert signatures to recoverable format (sorting by signer weight/address)
fn to_recoverable(_digest: Hash, mut signers_with_sigs: Vec<SignerWithSig>) -> Vec<SignerWithSig> {
    // Sort signers by their public key bytes for consistent ordering
    signers_with_sigs.sort_by_key(|s| s.signer.pub_key.clone());
    signers_with_sigs
}

pub fn payload_digest(
    domain_separator: &Felt,
    signer: &VerifierSet,
    payload: &Payload,
) -> Result<Hash, ContractError> {
    let signer_hash = StarknetWeightedSigners::try_from(signer)
        .map(|signers| signer_hash_poseidon(&signers))
        .change_context(ContractError::InvalidVerifierSet)?;

    let data_hash = data_hash_poseidon(&encode_payload_to_felts(payload)?);

    // Domain separator is already validated to be a valid felt during instantiation
    let digest_felt = payload_digest_poseidon(*domain_separator, signer_hash, data_hash);

    // Convert felt back to Hash
    let digest_bytes = digest_felt.to_bytes_be();
    Ok(Hash::from(digest_bytes))
}

fn encode_payload_to_felts(payload: &Payload) -> Result<Vec<Felt>, ContractError> {
    let command_type = StarknetCommandType::from(payload);
    let mut elements = command_type.cairo_serialize();

    match payload {
        Payload::Messages(messages) => {
            let starknet_messages: Result<Vec<StarknetMessage>, _> =
                messages.iter().map(StarknetMessage::try_from).collect();

            let starknet_messages =
                starknet_messages.change_context(ContractError::InvalidMessage)?;

            elements.push(Felt::from(starknet_messages.len() as u32));
            for message in &starknet_messages {
                elements.extend(message.cairo_serialize());
            }
        }
        Payload::VerifierSet(verifier_set) => {
            let weighted_signers = StarknetWeightedSigners::try_from(verifier_set)
                .change_context(ContractError::InvalidVerifierSet)?;

            elements.extend(weighted_signers.cairo_serialize());
        }
    }

    Ok(elements)
}

pub fn encode_execute_data(
    domain_separator: &Felt,
    verifier_set: &VerifierSet,
    signers: Vec<SignerWithSig>,
    payload: &Payload,
) -> Result<HexBinary, ContractError> {
    let signers = to_recoverable(
        payload_digest(domain_separator, verifier_set, payload)?,
        signers,
    );

    // Convert to Starknet types
    let weighted_signers = StarknetWeightedSigners::try_from(verifier_set)
        .change_context(ContractError::InvalidVerifierSet)?;

    let signatures: Vec<StarkSignature> = signers
        .into_iter()
        .map(StarkSignature::try_from)
        .collect::<Result<Vec<_>, _>>()
        .change_context(ContractError::Proof)?;

    let proof = StarknetProof {
        signers: weighted_signers,
        signatures,
    };

    let mut elements = Vec::new();

    elements.extend(encode_payload_to_felts(payload).unwrap());
    elements.extend(proof.cairo_serialize());

    let bytes = elements
        .into_iter()
        .flat_map(|felt| felt.to_bytes_be().to_vec())
        .collect::<Vec<u8>>();

    Ok(bytes.into())
}

#[cfg(test)]
mod tests {
    use assert_ok::assert_ok;
    use cosmwasm_std::HexBinary;
    use itertools::Itertools;
    use multisig::key::KeyTyped;
    use multisig::msg::{Signer, SignerWithSig};

    use crate::encoding::abi::tests::signers_with_sigs;
    use crate::encoding::starknet_cairo::{encode_execute_data, payload_digest};
    use crate::payload::Payload;
    use crate::test::test_data::{
        starknet_domain_separator, starknet_messages, verifier_set_from_pub_keys,
    };

    #[test]
    fn starknet_cairo_verifier_set_payload_digest() {
        let new_pub_keys = vec![
            "00a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc",
            "01ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca",
        ];
        let mut new_verifier_set = verifier_set_from_pub_keys(new_pub_keys);
        new_verifier_set.created_at = 75892034;

        let pub_keys = vec![
            "00a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc",
            "01ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca",
            "0411494b501a98abd8262b0da1351e17899a0c4ef23dd2f96fec5ba847310b20",
            "0759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf5",
            "0788435d61046d3eec54d77d25bd194525f4fa26ebe6575536bc6f656656b74c",
        ];

        let mut verifier_set = verifier_set_from_pub_keys(pub_keys);
        verifier_set.created_at = 75892033;

        let payload_digest = assert_ok!(payload_digest(
            &starknet_types_core::felt::Felt::from_bytes_be(&starknet_domain_separator()),
            &verifier_set,
            &Payload::VerifierSet(new_verifier_set),
        ));

        // Note: This will be a different hash value due to Poseidon instead of Keccak
        // The actual value will need to be verified against a Cairo implementation
        assert_eq!(
            hex::encode(payload_digest),
            "05c420cebe2ade8ae3827d733a1a84ba3136783aa26e6cfd777324d249c75dee"
        );
    }

    #[test]
    fn starknet_cairo_approve_messages_payload_digest() {
        let pub_keys = vec![
            "00a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc",
            "01ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca",
            "0411494b501a98abd8262b0da1351e17899a0c4ef23dd2f96fec5ba847310b20",
            "0759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf5",
            "0788435d61046d3eec54d77d25bd194525f4fa26ebe6575536bc6f656656b74c",
        ];

        let mut verifier_set = verifier_set_from_pub_keys(pub_keys);
        verifier_set.created_at = 75892033;

        let payload_digest = assert_ok!(payload_digest(
            &starknet_types_core::felt::Felt::from_bytes_be(&starknet_domain_separator()),
            &verifier_set,
            &Payload::Messages(starknet_messages()),
        ));

        assert_eq!(
            hex::encode(payload_digest),
            "054d0d7230d11874db95316d45fad69eff94d6667f3e1de28775cb52ae8dbb00"
        );
    }

    #[test]
    fn starknet_cairo_rotate_signers_execute_data() {
        let domain_separator = starknet_domain_separator();

        let new_pub_keys = vec![
            "00a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc",
            "01ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca",
        ];
        let mut new_verifier_set = verifier_set_from_pub_keys(new_pub_keys);
        new_verifier_set.created_at = 75892034;

        let pub_keys = vec![
            "00a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc",
            "01ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca",
            "0411494b501a98abd8262b0da1351e17899a0c4ef23dd2f96fec5ba847310b20",
            "0759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf5",
            "0788435d61046d3eec54d77d25bd194525f4fa26ebe6575536bc6f656656b74c",
        ];
        let mut verifier_set = verifier_set_from_pub_keys(pub_keys);
        verifier_set.created_at = 75892033;

        // Generated signatures from int tests
        let sigs: Vec<_> = vec![
        "07ef4d28a426d5cf032a713a56fefb6d39e779d7622871018c06d61cf92f702700a4ce6c80b56f2fb47dd4ab0021f31dba47f8d52aba709759fbe0a764ff3eff0000000000000000000000000000000000000000000000000000000000000001",
        "00da56fa3500fc7a1d0569dbc4a7d3c484433892c4439eb0a4f3b936fcf189f105e28493646e6ffb0a9ee8a7cab21e07379e11f5c69db4959800843eaf8a9a950000000000000000000000000000000000000000000000000000000000000001",
        "0568c83a818d03722192d52897582725a9b91bb6a5d02d56a3493590d3d1441a0535cdddc530d84f0974878e947f3efa0f6750d9afa6dd977ae881af02d3f2340000000000000000000000000000000000000000000000000000000000000001",
        "06390a8cbddf8981ae0e5b7fccfabe07fe7f4e35c0ef9447f5b25a06b5068b210219520abd72d2482efa65bc77e1304d62ab552c42b41e5e91b3b875e8e1eaa60000000000000000000000000000000000000000000000000000000000000001",
        "04f43e035c5ad139f661b503b182822bb952889cd4d420bb942b9a2c6c552a5705064b85a838aaa82090f778c8bde17fc33b517e55e98b08ec506bc10587137c0000000000000000000000000000000000000000000000000000000000000001",

        ].into_iter().map(|sig| HexBinary::from_hex(sig).unwrap()).collect();

        let signers_with_sigs = stark_signers_with_sigs(verifier_set.signers.values(), sigs);

        let payload = Payload::VerifierSet(new_verifier_set);

        let execute_data = assert_ok!(encode_execute_data(
            &starknet_types_core::felt::Felt::from_bytes_be(&domain_separator),
            &verifier_set,
            signers_with_sigs,
            &payload
        ));

        // Generated execute data from int tests
        assert_eq!(hex::encode(execute_data), "0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc000000000000000000000000000000000000000000000000000000000000000101ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000048605420000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000500a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc000000000000000000000000000000000000000000000000000000000000000101ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca00000000000000000000000000000000000000000000000000000000000000010411494b501a98abd8262b0da1351e17899a0c4ef23dd2f96fec5ba847310b2000000000000000000000000000000000000000000000000000000000000000010759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf500000000000000000000000000000000000000000000000000000000000000010788435d61046d3eec54d77d25bd194525f4fa26ebe6575536bc6f656656b74c0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000048605410000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000507ef4d28a426d5cf032a713a56fefb6d39e779d7622871018c06d61cf92f702700a4ce6c80b56f2fb47dd4ab0021f31dba47f8d52aba709759fbe0a764ff3eff000000000000000000000000000000000000000000000000000000000000000100da56fa3500fc7a1d0569dbc4a7d3c484433892c4439eb0a4f3b936fcf189f105e28493646e6ffb0a9ee8a7cab21e07379e11f5c69db4959800843eaf8a9a9500000000000000000000000000000000000000000000000000000000000000010568c83a818d03722192d52897582725a9b91bb6a5d02d56a3493590d3d1441a0535cdddc530d84f0974878e947f3efa0f6750d9afa6dd977ae881af02d3f234000000000000000000000000000000000000000000000000000000000000000106390a8cbddf8981ae0e5b7fccfabe07fe7f4e35c0ef9447f5b25a06b5068b210219520abd72d2482efa65bc77e1304d62ab552c42b41e5e91b3b875e8e1eaa6000000000000000000000000000000000000000000000000000000000000000104f43e035c5ad139f661b503b182822bb952889cd4d420bb942b9a2c6c552a5705064b85a838aaa82090f778c8bde17fc33b517e55e98b08ec506bc10587137c0000000000000000000000000000000000000000000000000000000000000001");
    }

    #[test]
    fn starknet_cairo_approve_messages_execute_data() {
        let domain_separator = starknet_domain_separator();

        let pub_keys = vec![
            "00a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc",
            "01ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca",
            "0411494b501a98abd8262b0da1351e17899a0c4ef23dd2f96fec5ba847310b20",
            "0759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf5",
            "0788435d61046d3eec54d77d25bd194525f4fa26ebe6575536bc6f656656b74c",
        ];
        let mut verifier_set = verifier_set_from_pub_keys(pub_keys);
        verifier_set.created_at = 75892033;

        // Generated signatures from the int tests
        let sigs: Vec<_> = vec![
        "068659b58c0a3017c462f6629ef10958ce6c5e06bf0eb5550915a729f654c5350234799de7dd210888917eed841b18245cd9f465c52ef28ac89dcbb0776a8b390000000000000000000000000000000000000000000000000000000000000000",
         "0770a33529537812cb628225a64d393b960d5c1b42e53845365cddc48abf1a57033747300c7bf5b7574a57e9aec372de0bc2068ed5d0154bd8e07398aa15de150000000000000000000000000000000000000000000000000000000000000001",
         "00e65a95ac3803a1d828622276c7f4442f708e471f33aa5b9a9aec5d777540e005096c97cb0efdfa2dcd8f43a41925a17182c24054bcecd4d072eab3672b4f6b0000000000000000000000000000000000000000000000000000000000000000",
         "016c026d82c78dd4912f1ed464ed5b4676a43c4c4657b4d98b9574005ef0b7990157841942cbf98ffbcd084a2f56117e1a685c8e22277215b28d24c5058685080000000000000000000000000000000000000000000000000000000000000001",
         "014bc525cf9de45b2740409c06c29c7265b2e78d6df1fd21b0997fac178c88e804abea879b69468b7481929355faa7e146132372725ceef4facdb689748d8fb30000000000000000000000000000000000000000000000000000000000000001"
        ].into_iter().map(|sig| HexBinary::from_hex(sig).unwrap()).collect();

        let signers_with_sigs = stark_signers_with_sigs(verifier_set.signers.values(), sigs);

        let payload = Payload::Messages(starknet_messages());
        let execute_data = assert_ok!(encode_execute_data(
            &starknet_types_core::felt::Felt::from_bytes_be(&domain_separator),
            &verifier_set,
            signers_with_sigs,
            &payload
        ));

        // Generated execute_data from the int tests
        assert_eq!(hex::encode(execute_data), "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000657468657265756d0000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000074785f69645f31307369672d6576656e745f6964785f300000000000000000000000000000000000000000000000000000000000000017000000000000000000000000000000000000000000000000000000000000000100307837314337363536454337616238386230393864656642373531423734300000000000000000000000000000000000000000003142356636643839373646000000000000000000000000000000000000000000000000000000000000000b0402de8f0afb2615e0889bc1ff92bf2fb32562f541bd4301d393269aec0f2dae000000000000000000000000000000005eb6269017843215ecaa19f56ccffdcb0000000000000000000000000000000025e41f1a98129e1482eca0b377ff8140000000000000000000000000000000000000000000000000000000000000000500a7da05a4d664859ccd6e567b935cdfbfe3018c7771cb980892ef38878ae9bc000000000000000000000000000000000000000000000000000000000000000101ef15c18599971b7beced415a40f0c7deacfd9b0d1819e03d723d8bc943cfca00000000000000000000000000000000000000000000000000000000000000010411494b501a98abd8262b0da1351e17899a0c4ef23dd2f96fec5ba847310b2000000000000000000000000000000000000000000000000000000000000000010759ca09377679ecd535a81e83039658bf40959283187c654c5416f439403cf500000000000000000000000000000000000000000000000000000000000000010788435d61046d3eec54d77d25bd194525f4fa26ebe6575536bc6f656656b74c00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000486054100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005068659b58c0a3017c462f6629ef10958ce6c5e06bf0eb5550915a729f654c5350234799de7dd210888917eed841b18245cd9f465c52ef28ac89dcbb0776a8b3900000000000000000000000000000000000000000000000000000000000000000770a33529537812cb628225a64d393b960d5c1b42e53845365cddc48abf1a57033747300c7bf5b7574a57e9aec372de0bc2068ed5d0154bd8e07398aa15de15000000000000000000000000000000000000000000000000000000000000000100e65a95ac3803a1d828622276c7f4442f708e471f33aa5b9a9aec5d777540e005096c97cb0efdfa2dcd8f43a41925a17182c24054bcecd4d072eab3672b4f6b0000000000000000000000000000000000000000000000000000000000000000016c026d82c78dd4912f1ed464ed5b4676a43c4c4657b4d98b9574005ef0b7990157841942cbf98ffbcd084a2f56117e1a685c8e22277215b28d24c5058685080000000000000000000000000000000000000000000000000000000000000001014bc525cf9de45b2740409c06c29c7265b2e78d6df1fd21b0997fac178c88e804abea879b69468b7481929355faa7e146132372725ceef4facdb689748d8fb30000000000000000000000000000000000000000000000000000000000000001");
    }

    pub(crate) fn stark_signers_with_sigs<'a>(
        signers: impl Iterator<Item = &'a Signer>,
        sigs: Vec<HexBinary>,
    ) -> Vec<SignerWithSig> {
        signers
            .sorted_by(|s1, s2| Ord::cmp(&s1.pub_key, &s2.pub_key))
            .zip(sigs)
            .map(|(signer, sig)| {
                signer.with_sig(
                    multisig::key::Signature::try_from((signer.pub_key.key_type(), sig)).unwrap(),
                )
            })
            .collect()
    }
}
