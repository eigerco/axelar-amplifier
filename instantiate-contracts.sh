#!/usr/bin/env bash

# instantiate contracts to start axelar network

export ADMIN_ADDRESS=$(axelard keys show admin -a --keyring-backend test)
export VALIDATOR_ADDRESS=$(axelard keys show validator -a --keyring-backend test)
export GOVERNANCE_ADDRESS=$(axelard keys show governance -a --keyring-backend test)

export CHAN_ID=axelar-testnet-local
# export ADMIN_ADDRESS=$VALIDATOR_ADDRESS
# export GOVERNANCE_ADDRESS=$VALIDATOR_ADDRESS

export FAKE_ADDRESS=axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9
source code_ids.sh

rm -rf instantiate-logs
mkdir -p instantiate-logs

# start service registry
axelard tx wasm instantiate $SERVICE_REGISTRY_CODE_ID '{ "governance_account": "'"$GOVERNANCE_ADDRESS"'" }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-service-registry \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-service-registry.log

export SERVICE_REGISTRY_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-service-registry.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

axelard tx wasm instantiate $REWARDS_CODE_ID \
  '{
    "governance_address": "'"$GOVERNANCE_ADDRESS"'",
    "rewards_denom":"uwasm"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-rewards \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-rewards.log

export REWARDS_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-rewards.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

export AXELAR_NET_GATEWAY=$FAKE_ADDRESS
# TODO set gxelarnet gateway address
axelard tx wasm instantiate $ROUTER_CODE_ID \
  '{
    "admin_address":"'"$ADMIN_ADDRESS"'",
    "governance_address":"'"$GOVERNANCE_ADDRESS"'",
    "axelarnet_gateway":"'"$AXELAR_NET_GATEWAY"'"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-router \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-router.log

export ROUTER_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-router.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

export NEXUS_ADDRESS=$FAKE_ADDRESS
# TODO set gxelarnet gateway address
axelard tx wasm instantiate $AXELARNET_GATEWAY_CODE_ID \
  '{
    "chain_name": "aleo",
    "router_address": "'"$ROUTER_ADDRESS"'",
    "nexus": "'"$NEXUS_ADDRESS"'"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-axelar-gateway \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-axelar-gateway.log
export AXELARNET_GATEWAY_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-axelar-gateway.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

axelard tx wasm instantiate $COORDINATOR_CODE_ID \
  '{
    "governance_address": "'"$GOVERNANCE_ADDRESS"'",
    "service_registry": "'"$SERVICE_REGISTRY_ADDRESS"'"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-coordinator \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-coordinator.log
export COORDINATOR_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-coordinator.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

axelard tx wasm instantiate $MULTISIG_CODE_ID \
  '{
      "governance_address": "'"$GOVERNANCE_ADDRESS"'",
      "admin_address": "'"$ADMIN_ADDRESS"'",
      "rewards_address": "'"$REWARDS_ADDRESS"'",
      "block_expiry": "10"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-multisig \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-multisig.log
export MULTISIG_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-multisig.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

export MY_SOURCE_CHAIN_GATEWAY_ADDRESS="vzevxifdoj.aleo"
# TODO set service_registry_address
# TODO set rewards_address
export MSG_ID_FORMAT=hex_tx_hash_and_event_index
axelard tx wasm instantiate $VOTING_VERIFIER_CODE_ID \
  '{
    "governance_address":"'"$GOVERNANCE_ADDRESS"'",
    "service_registry_address":"'"$SERVICE_REGISTRY_ADDRESS"'",
    "service_name":"validators",
    "source_gateway_address":"'"$MY_SOURCE_CHAIN_GATEWAY_ADDRESS"'",
    "voting_threshold":["1","1"],
    "block_expiry":"10",
    "confirmation_height":1,
    "source_chain":"aleo",
    "rewards_address":"'"$REWARDS_ADDRESS"'",
    "msg_id_format":"'"$MSG_ID_FORMAT"'",
    "address_format":"aleo"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-voting-verifier \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-voting-verifier.log
export VERIFIER_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-voting-verifier.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

axelard tx wasm instantiate $GATEWAY_CODE_ID \
  '{
    "verifier_address": "'"$VERIFIER_ADDRESS"'",
    "router_address": "'"$ROUTER_ADDRESS"'"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-gateway \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-gateway.log

export GATEWAY_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-gateway.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

# export GATEWAY_ADDRESS=TODO
# export VERIFIER_ADDRESS=TODO
# export MULTISIG_ADDRESS=TODO
# export COORDINATOR_ADDRESS=TODO
# export SERVICE_REGISTRY_ADDRESS=TODO
# export GOVERNANCE_ADDRESS=TODO
# export MY_CHAIN_ID=TODO # this is on their docs but not used
axelard tx wasm instantiate $MULTISIG_PROVER_CODE_ID \
  '{
      "admin_address": "'"$ADMIN_ADDRESS"'",
      "governance_address": "'"$GOVERNANCE_ADDRESS"'",
      "gateway_address": "'"$GATEWAY_ADDRESS"'",
      "multisig_address": "'"$MULTISIG_ADDRESS"'",
      "coordinator_address": "'"$COORDINATOR_ADDRESS"'",
      "service_registry_address": "'"$SERVICE_REGISTRY_ADDRESS"'",
      "voting_verifier_address": "'"$VERIFIER_ADDRESS"'",
      "signing_threshold": ["1","1"],
      "service_name": "validators",
      "chain_name":"test",
      "verifier_set_diff_threshold": 1,
      "encoder": "abi",
      "key_type": "ecdsa",
      "domain_separator": "6973c72935604464b28827141b0a463af8e3487616de69c5aa0c785392c9fb9f"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-multisig-prover \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-multisig-prover.log
export MULTISIG_PROVER_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-multisig-prover.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

axelard tx wasm instantiate $INTERCHAIN_TOKEN_SERVICE_CODE_ID \
  '{
    "governance_address": "'"$GOVERNANCE_ADDRESS"'",
    "admin_address": "'"$ADMIN_ADDRESS"'",
    "axelarnet_gateway_address": "'"$AXELAR_NET_GATEWAY"'"
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --label test-interchain-token \
  --admin "$ADMIN_ADDRESS" \
  --broadcast-mode block \
  --yes 2>&1 | tee instantiate-logs/instantiate-interchain-token.log
export INTERCHAIN_TOKEN_SERVICE_ADDRESS=$(sed -n '2p' instantiate-logs/instantiate-interchain-token.log | jq -r '.logs[0].events[0].attributes[] | select(.key == "_contract_address") | .value')

axelard tx wasm execute $ROUTER_ADDRESS \
  '{
      "axelar_gateway": {
          "axelarnet_gateway": "'"$AXELARNET_GATEWAY_ADDRESS"'"
      }
  }' \
  --keyring-backend test \
  --from validator \
  --gas auto \
  --gas-adjustment 1.5 \
  --chain-id axelar-testnet-local \
  --broadcast-mode block \
  --yes

