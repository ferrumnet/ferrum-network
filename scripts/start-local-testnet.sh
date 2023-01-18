
# remove any existing data from chain
rm -rf ./chain

# generate chain spec
./target/release/ferrum-x-network build-spec --disable-default-bootnode > ferrum-local-testnet.json

# insert the signing keys for alice
./target/release/ferrum-x-network key insert --key-type ofsg --scheme ecdsa --base-path ./chain/alice --chain ferrum-local-testnet.json --suri //Alice

# insert the signing keys for bob
./target/release/ferrum-x-network key insert --key-type ofsg --scheme ecdsa --base-path ./chain/bob --chain ferrum-local-testnet.json --suri //bob

# start Alice node
./target/release/ferrum-x-network --chain ferrum-local-testnet.json --alice --base-path ./chain/alice --ws-port 9944 --config-file-path ./alice_node_config.json

# start Bob node
# ./target/release/ferrum-x-network --chain ferrum-local-testnet.json --bob --base-path ./chain/bob --ws-port 9945 --config-file-path ./bob_node_config.json
