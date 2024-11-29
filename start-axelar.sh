#!/usr/bin/env bash

mkdir -p /tmp/axelar.conf
cp -r -a ~/.axelar/* /tmp/axelar.conf/.
rm -rf ~/.axelar

CHAIN_ID="axelar-testnet-local"

axelard init mynode --chain-id $CHAIN_ID

axelard keys add validator --keyring-backend test
axelard add-genesis-account "$(axelard keys show validator -a --keyring-backend test)" 10000000000stake,10000000000uaxl

axelard keys add governance --keyring-backend test
axelard keys add admin --keyring-backend test

axelard gentx validator 10000000000stake \
  --chain-id $CHAIN_ID \
  --keyring-backend test

axelard gentx governance 10000000000stake \
  --chain-id $CHAIN_ID \
  --keyring-backend test

axelard gentx admin 10000000000stake \
  --chain-id $CHAIN_ID \
  --keyring-backend test

axelard collect-gentxs

axelard validate-genesis
axelard config chain-id $CHAIN_ID


jq '.app_state.wasm.params.code_upload_access.permission = "Everybody"'  ~/.axelar/config/genesis.json  > temp.json && mv temp.json ~/.axelar/config/genesis.json
jq '.app_state.wasm.params.instantiate_default_permission = "Everybody"'  ~/.axelar/config/genesis.json  > temp.json && mv temp.json ~/.axelar/config/genesis.json
# jq '.app_state.permission.gov_accounts = ["'"$VALIDATOR_ADDRESS"'", "'"$GOVERNANCE_ADDRESS"'"]'  ~/.axelar/config/genesis.json  > temp.json && mv temp.json ~/.axelar/config/genesis.json

sed -i 's/max_body_bytes = 1000000/max_body_bytes = 100000000/' ~/.axelar/config/config.toml
sed -i 's/rpc-max-body-bytes = 1000000/rpc-max-body-bytes = 100000000/' ~/.axelar/config/app.toml

# DO THIS MANUALLY
# export VALIDATOR_ADDRESS=$(axelard keys show validator -a --keyring-backend test)
# export GOVERNANCE_ADDRESS=$(axelard keys show governance -a --keyring-backend test)
# echo $VALIDATOR_ADDRESS
# echo $GOVERNANCE_ADDRESS
# Set the above values in the genesis.json file
# ~/.axelar/config/genesis.json

# cp /tmp/axelar.conf/config/app.toml ~/.axelar/config/app.toml
# cp /tmp/axelar.conf/config/config.toml ~/.axelar/config/config.toml

# axelard start 2>&1 | tee /tmp/local-axelar.log






