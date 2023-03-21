#!/bin/bash
./target/release/ferrum-network build-spec --disable-default-bootnode --chain alpha-testnet > ./chainspecs/ferrum-alpha-testnet.json


# Relay 1
./target/release/polkadot --chain rococo-local-cfde.json --alice --tmp

# Relay 2
./target/release/polkadot --chain rococo-local-cfde.json --bob --tmp --port 30334




./target/release/ferrum-network export-genesis-state --chain dev > genesis-state


./target/release/ferrum-network export-genesis-wasm --chain dev > genesis-wasm



# Collator1
./target/release/ferrum-network --collator --alice --force-authoring --dev --tmp --port 40335 \
--ws-port 9946 -- --execution wasm --chain ../../polkadot/rococo-local-cfde.json --port 30335
