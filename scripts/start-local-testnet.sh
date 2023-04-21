
# # remove any existing data from chain
# rm -rf ./chain

# # generate chain spec
# ./target/release/ferrum-network build-spec --disable-default-bootnode > ferrum-local-testnet.json

# # insert the signing keys for alice
# ./target/release/ferrum-network key insert --key-type ofsg --scheme ecdsa --base-path ./chain/alice --chain ferrum-local-testnet.json --suri //Alice

# # insert the signing keys for bob
# ./target/release/ferrum-network key insert --key-type ofsg --scheme ecdsa --base-path ./chain/bob --chain ferrum-local-testnet.json --suri //Bob

# start relaychain and parachain in background
polkadot-launch ./scripts/polkadot-launch/config.json