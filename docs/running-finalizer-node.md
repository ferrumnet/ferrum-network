# Running a QP finalizer node

<!> This guide assumes you have followed the instructions to setup your node [here](running-nodes.md).

<!> External miners are not supported in the current ferrum testnet, we will announce once we start onboarding finalizers!.

## Table of Contents:

* [Prerequisites](#prerequisites)
* [Run using docker](#run-using-docker)
* [Run using binary](#run-using-binary)

## Prerequisites

### Setup Config file 

Running a QP node requires setting up the configuration for the sourcechain and destinationchain pairs, it is essential to set these values up correctly or the transactions will fail.

These are the values to be included in the file : 

```json
{
    "networks": {
        "network_vec": [{
            "url": "<CHAIN_1_RPC_URL>",
            "gateway_contract_address": "<CHAIN_1_LEDGER_MGR_ADDRESS>",
            "id": "CHAIN_1_ID"
        },
        {
            "url": "<CHAIN_2_RPC_URL>",
            "gateway_contract_address": "<CHAIN_2_LEDGER_MGR_ADDRESS>",
            "id": "CHAIN_2_ID"
        }],
	    "pair_vec": [[CHAIN_1_ID, CHAIN_2_ID], [CHAIN_1_ID, CHAIN_2_ID]],
        "signer_public_key": "PUB_KEY",
        "role" : "<NODE_ROLE>"
    }
}
```

Values explained : 

- The network_vec contains the list of chains that the miner/finaliser will connect to, this should include the url of the chain, the qp ledger manager address and the chain id of the respective chain.
- signer_public_key contains the public key of the address you will use to sign the mine/finalise transactions, do ensure that the address has balance to execute the transactions on both chains.
- authority_manager_contract_addres refers to the address of the QP authority manager contract.
- NODE_ROLE signifies the role you choose for the node, it should be one of `QP_FINALIZER` or `QP_MINER`

An example config for finalizing between Mumbai testnet and BSC testnet would be as follows : 

```json
{
    "networks": {
        "network_vec": [{
            "url": "https://rpc-mumbai.maticvigil.com/",
            "gateway_contract_address": "1AC9Fb66D542FEe49728e0da6af230dbECD6d547",
            "id": 80001
        },
        {
            "url": "https://data-seed-prebsc-2-s3.binance.org:8545",
            "gateway_contract_address": "1AC9Fb66D542FEe49728e0da6af230dbECD6d547",
            "id": 97
        }],
	    "pair_vec": [[80001, 97], [97, 80001]],
        "signer_public_key": "0x000000000000",
        "role" : "QP_FINALIZER"
    }
}
```

Setup the config and save it to a file called `node-config.json`

### Setup signer keys

Follow the instructions [here](running-nodes.md)to generate a signer key, this key will be signing and mining all transactions, so make sure you store it carefully.

## Run using docker

Follow the below instructions, to setup your miner node to connect to Ferrum testnet

1. Build the docker image using:

```bash
docker build -t ferrum_node -f docker/ferrum.Dockerfile .
```

2. Next, make sure you set the ownership and permissions accordingly for the local directory that stores the chain data. In this case, set the necessary permissions either for a specific or current user (replace DOCKER_USER for the actual user that will run the docker command):

```bash
# chown to a specific user
mkdir /var/lib/ferrum-data
chown DOCKER_USER /var/lib/ferrum-data

# chown to current user
sudo chown -R $(id -u):$(id -g) /var/lib/ferrum-data
```

3. Before you can start the node, you have to insert the keys

    You need to insert the ofsg key for the account to sign and post transactions on the QP chains

    ```bash
    docker run --network="host" -v "/var/lib/ferrum-data:/data" \
    ferrum/ferrum_node:latest \
    key insert --key-type ofsg --scheme Ecdsa --base-path=/data
    ```

    The CLI will ask you to enter the keys, enter the key generated in the previous step.

4. Now, execute the docker run command depending on your configuration : 

```bash
docker run --network="host" -v "/var/lib/ferrum-data:/data" \
-u $(id -u ${USER}):$(id -g ${USER}) \
ferrum_node \
--base-path=/data \
--chain alpha-testnet \
--name="YOUR-NODE-NAME" \
--config-file-path=/var/lib/node-config.json
-- \
--execution wasm \
--name="YOUR-NODE-NAME (Embedded Relay)"
```
Once the node has started, your output should look similar to this

```bash
2023-04-28 17:22:41 Ferrum Parachain    
2023-04-28 17:22:41 ✌️  version 0.0.1-742b47b9d10    
2023-04-28 17:22:41 ❤️  by Ferrum Network <https://github.com/ferrumnet>, 2020-2023    
2023-04-28 17:22:41 📋 Chain specification: Ferrum Testnet    
2023-04-28 17:22:41 🏷  Node name: TestNode    
2023-04-28 17:22:41 👤 Role: AUTHORITY    
2023-04-28 17:22:41 💾 Database: RocksDb at ./chain/alice/chains/ferrum_testnet/db/full    
2023-04-28 17:22:41 ⛓  Native runtime: ferrum-parachain-1 (ferrum-parachain-0.tx1.au1)    
2023-04-28 17:22:43 assembling new collators for new session 0 at #0    
2023-04-28 17:22:43 assembling new collators for new session 1 at #0    
2023-04-28 17:22:43 Parachain id: Id(1000)    
2023-04-28 17:22:43 Parachain Account: 5Ec4AhPZk8STuex8Wsi9TwDtJQxKqzPJRCH7348Xtcs9vZLJ    
2023-04-28 17:22:43 Parachain genesis state: 0x000000000000000000000000000000000000000000000000000000000000000000cb981b199b0dfb2631bbac63b767890daad314c0ce7b0d681e0fa76354a9b89803170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c11131400    
2023-04-28 17:22:43 Is collating: yes    
2023-04-28 17:22:43 [Parachain] assembling new collators for new session 0 at #0    
2023-04-28 17:22:43 [Parachain] assembling new collators for new session 1 at #0    
```

Depending on how long the testnet has been running, your node will take a while to sync with the latest state of the network.

## Run using binary

1. Install the required dependencies to compile rust and substrate, refer the documentation here : https://docs.substrate.io/install/

2. Clone the ferrum-network repo

```bash
https://github.com/ferrumnet/ferrum-network.git
```

3. Checkout the latest release

```bash
cd ferrum-network
git checkout tags/<release_version> -b <release_version>
```

For example, if the latest release is 0.0.3

```bash
git checkout tags/0.0.3 -b v0.0.3
```


You can checkout releases here : https://github.com/ferrumnet/ferrum-network/releases

4. Build the binary

```
cargo build --release
```

5. Insert the keys

```bash
./target/release/ferrum-network key insert --key-type ofsg --scheme Ecdsa --base-path /var/lib/ferrum-data
```

6. Once the keys are inserted, you can run it using the following command

```bash
./target/release/ferrum-network \
--base-path=/var/lib/ferrum-data \
--chain alpha-testnet \
--name="YOUR-NODE-NAME" \
-config-file-path node-config.json
-execution wasm \
-- \
--execution wasm \
--name="YOUR-NODE-NAME (Embedded Relay)"
```

Once the node has started, your output should look similar to this

```bash
2023-04-28 17:22:41 Ferrum Parachain    
2023-04-28 17:22:41 ✌️  version 0.0.1-742b47b9d10    
2023-04-28 17:22:41 ❤️  by Ferrum Network <https://github.com/ferrumnet>, 2020-2023    
2023-04-28 17:22:41 📋 Chain specification: Ferrum Testnet    
2023-04-28 17:22:41 🏷  Node name: TestNode    
2023-04-28 17:22:41 👤 Role: AUTHORITY    
2023-04-28 17:22:41 💾 Database: RocksDb at ./chain/alice/chains/ferrum_testnet/db/full    
2023-04-28 17:22:41 ⛓  Native runtime: ferrum-parachain-1 (ferrum-parachain-0.tx1.au1)    
2023-04-28 17:22:43 assembling new collators for new session 0 at #0    
2023-04-28 17:22:43 assembling new collators for new session 1 at #0    
2023-04-28 17:22:43 Parachain id: Id(1000)    
2023-04-28 17:22:43 Parachain Account: 5Ec4AhPZk8STuex8Wsi9TwDtJQxKqzPJRCH7348Xtcs9vZLJ    
2023-04-28 17:22:43 Parachain genesis state: 0x000000000000000000000000000000000000000000000000000000000000000000cb981b199b0dfb2631bbac63b767890daad314c0ce7b0d681e0fa76354a9b89803170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c11131400    
2023-04-28 17:22:43 Is collating: yes    
2023-04-28 17:22:43 [Parachain] assembling new collators for new session 0 at #0    
2023-04-28 17:22:43 [Parachain] assembling new collators for new session 1 at #0    
```
Depending on how long the testnet has been running, your node will take a while to sync with the latest state of the network.