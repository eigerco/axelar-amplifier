use coordinator::msg::ChainContractsKey;

pub mod test_utils;

#[test]
fn chain_contracts_information_should_be_consistent_in_coordinator() {
    let test_utils::TestCase {
        mut protocol,
        chain1: ethereum,
        ..
    } = test_utils::setup_test_case();

    let ethereum_by_chain_name = test_utils::chain_contracts_info_from_coordinator(
        &mut protocol,
        ChainContractsKey::ChainName(ethereum.chain_name.clone()),
    );

    goldie::assert_json!(ethereum_by_chain_name);

    let ethereum_by_gateway = test_utils::chain_contracts_info_from_coordinator(
        &mut protocol,
        ChainContractsKey::GatewayAddress(ethereum.gateway.contract_addr.clone()),
    );

    goldie::assert_json!(ethereum_by_gateway);

    let ethereum_by_prover = test_utils::chain_contracts_info_from_coordinator(
        &mut protocol,
        ChainContractsKey::ProverAddress(ethereum.multisig_prover.contract_addr.clone()),
    );

    goldie::assert_json!(ethereum_by_prover);

    let ethereum_by_verifier = test_utils::chain_contracts_info_from_coordinator(
        &mut protocol,
        ChainContractsKey::VerifierAddress(ethereum.voting_verifier.contract_addr.clone()),
    );

    goldie::assert_json!(ethereum_by_verifier);
}
