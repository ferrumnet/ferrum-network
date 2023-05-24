#!/bin/bash
echo "Generating artifacts for chainspec : $1";

# generate the raw chainspec
./target/release/ferrum-network build-spec --disable-default-bootnode --chain $1 --raw > ./chainspecs/$1.json

# generate the genesis state
./target/release/ferrum-network export-genesis-state --chain ./chainspecs/$1.json ./chainspecs/$1-state

# generate the wasm
./target/release/ferrum-network export-genesis-wasm --chain ./chainspecs/$1.json ./chainspecs/$1-wasm