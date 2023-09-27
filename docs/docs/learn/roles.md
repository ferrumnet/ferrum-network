## Introduction

Collators/collators are members of the network that maintain the parachains they take part in. They run a full node (for both their particular parachain and the relay chain), and they produce the state transition proof for relay chain collators.

Candidates will need a minimum amount of tokens bonded (self-bonded) to be considered eligible. Along with running a collator node for the ferrum blockchain, you can choose to run a specific type of Ferrum node, which help in validating cross chain transactions on the ferrum network.

## Types of Nodes

The different types of nodes of ferrum network:

1. **Collator node**

    Running a collator node means you pariticipate in the block production of ferrum network. Once your collator node is up and running, you can choose to be a block producer candidate, and if you have a minimum amount of tokens you would be selected for block production. Currently we do not have staking or rewards for block production but we plan to support this in the future.


2. **Miner Node (QP Miner)**
    
    A miner node is responsible for mining cross chain transactions, these nodes will observe the qp chain pairs and mine blocks on each other chains. This type of node can be run in conjunction with a collator node or indepdently to mine the block on the pair chain. Do note that running this node requires a minimum amount of tokens to pay for transaction costs on the pair chains.


3. **Finalizer Node (QP Finalizer)**

    The finalizer node is responsible for finalizing the mined blocks, these nodes will observe the mined blocks on the pair chains and finalize the block on the pair chain. This type of node can be run in conjunction with a collator node or indepdently to finalize the block on the pair chain. Do note that running this node requires a minimum amount of tokens to pay for transaction costs on the pair chains.

4. **Archive Node**

    The archive node is the simplest type of node, the node will sync and update the latest block on the ferrum chain. This type of node is useful to run an indexer or explorer.