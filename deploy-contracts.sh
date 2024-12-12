#!/usr/bin/env bash

export CHAIN_ID="axelar-testnet-local"

declare -A contracts=(
    [axelarnet_gateway]=""
    [gateway]=""
    [multisig]=""
    [service_registry]=""
    [interchain_token_service]=""
    [rewards]=""
    [voting_verifier]=""
    [coordinator]=""
    [multisig_prover]=""
    [router]=""
)

mkdir -p deploy-logs

for contract_name in "${!contracts[@]}"; do
  contract_file=${contract_name}.wasm
  echo "Deploying '$contract_file'"
  axelard tx wasm store artifacts/"$contract_file" \
      --keyring-backend test \
      --from validator \
      --chain-id $CHAIN_ID \
      --gas-adjustment 1.5 \
      --gas auto \
      --broadcast-mode block 2>&1 | tee deploy-logs/"$contract_name".log

  # read the output of the log file and get the code id
  code_id=$(sed -n '2p' deploy-logs/"$contract_name".log | jq -r '.logs[0].events[1].attributes[] | select(.key == "code_id") | .value')

  contracts[$contract_name]=$code_id

  echo "Code ID for '$contract_name' is $code_id"
done

echo "Exporting code IDs:"
CODE_IDS_FILE="code_ids.sh"
rm $CODE_IDS_FILE
touch $CODE_IDS_FILE
for contract_name in "${!contracts[@]}"; do
  upper_case_name=$(echo "$contract_name" | tr '[:lower:]' '[:upper:]')
  echo "export ${upper_case_name}_CODE_ID=${contracts[$contract_name]}" >> $CODE_IDS_FILE
done

axelard query wasm list-code

