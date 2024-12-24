#!/usr/bin/env bash

set -x
set -e

export CHAIN_ID="axelar-testnet-local"
export CONTRACT_NAME="multisig_prover"

echo "Optimize '$CONTRACT_NAME'"
./optimize_wasm.sh $CONTRACT_NAME

echo "Checking '$CONTRACT_NAME'"
cosmwasm-check artifacts/"$CONTRACT_NAME".wasm

echo "Deploying '$CONTRACT_NAME'"
axelard tx wasm store artifacts/multisig_prover.wasm \
  --keyring-backend test \
  --from wallet \
  --gas auto --gas-adjustment 1.5 --gas-prices 0.00005uamplifier \
  --chain-id devnet-amplifier \
  --node http://devnet-amplifier.axelar.dev:26657 2>&1 | tee "$CONTRACT_NAME".log

  # read the output of the log file and get the code id
code_id=$(sed -n '2p' "$CONTRACT_NAME".log | jq -r '.logs[0].events[1].attributes[] | select(.key == "code_id") | .value')

echo "Code ID for '$CONTRACT_NAME' is $code_id"

echo "Exporting code IDs:"
CODE_IDS_FILE="code_ids_$CONTRACT_NAME.sh"
rm $CODE_IDS_FILE || true
touch $CODE_IDS_FILE
upper_case_name=$(echo "$CONTRACT_NAME" | tr '[:lower:]' '[:upper:]')
echo "export ${upper_case_name}_CODE_ID=${code_id}" >> $CODE_IDS_FILE

export MULTISIG_PROVER_ADDRESS=axelar1xzv5w93nu8hvth4f409l07l9cgj7ga7de0dfegnzcqz7qnygya4s07u45r

axelard tx wasm migrate $MULTISIG_PROVER_ADDRESS "${code_id}" \
    '{ }' \
    --keyring-backend test \
    --from wallet \
    --gas auto --gas-adjustment 1.5 --gas-prices 0.00005uamplifier\
    --chain-id devnet-amplifier \
    --node http://devnet-amplifier.axelar.dev:26657 2>&1 | tee migrate_$CONTRACT_NAME.log

