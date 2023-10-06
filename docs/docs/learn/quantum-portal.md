## How Quantum Portal works

Quantum Portal is part of the Ferrum Runtime Node. When you deploy the Ferrum Network node you can configure it to mine or validate Quantum Portal transactions of Ferrum Network transactions as a collator on the network.

### Overview

Quantum Portal is part of the Ferrum Runtime Node. When you deploy the Ferrum Network node you can configure it to mine or validate Quantum Portal transactions of Ferrum Network transactions as a collator on the network.

Quantum Portal includes the following core components:

1. QP Smart Contract
2. QP Miner
3. QP Finalizer

![alt text](../images/qp-overview.png "Title")

### What is Quantum Portal Mining?

The QP Miners take turns based on an algorithm to create and relay these blocks from the sourceChain to the destinationChain. QP Miners do this by running the Ferrum Node as a QP Miner (QP Node). Once configured, this QP Node monitors the transactions on the network that they have set up to be miners and staked tokens on. The QP Node monitors transactions on the sourceChain and if new data is available, it creates a block every 15 seconds. After creating a block, the QP Node calls the mineRemoteBlock on the destinationChain in order to execute the transaction and mine the QP Block. It is considered a mined block after the transaction executes on the destinationChain

### What is Quantum Portal Finalisation?

The QP collators take turns based on an algorithm to pick the pending (mined but not finalized) Quantum Portal Blocks from the Quantum Portal Mined Block mempool. QP collators do this by running the Ferrum Node as a QP collator (QP Node). Once configured, this QP Node monitors the Quantum Portal Mined Block mempool for mined Quantum Portal Blocks, If new data is available, it creates a finalized block every 15 seconds. After creating a finalized block, the QP Node calls the finalizeRemoteBlock on the destinationChain in order to record the block as finalized and execute any remote transactions if applicable. The QP Block. It is considered a finalized block after the finalizeRemoteBlock transaction executes on the destinationChain
Once mined QP Blocks are finalized, the record of the finalized mined blocks and the finalized block itself is added to the destinationChains.