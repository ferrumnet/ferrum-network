# Miner Staking on Ferrum testnet

To become a miner on the ferrum testnet, you need to stake tFRM tokens, this stake is to ensure honest behavior of miners and to ensure equal participation to all miners on the network.

For security reasons, it is recommended that you do not use the account associated with your miner on any hot wallet (like metamask), the keys to your miner has be stored safely. To ensure the safety of your primary miner key, the Ferrum Miner Staking contract allows you to delegate to any miner of your choice.

This means that you can store your funds in a separate wallet and use that to stake, then delegate the stake to the miner account associated with your miner node.

The steps for this would be :

1. Stake on Ferrum mining manager contract
2. Delegate to miner address


To stake, you can use the testnet dashboard at https://testnet.faucet.ferrumnetwork.io/staking

1. To stake tFRM tokens, select the client network you wish to mine, navigate to the stake tab and enter the amount of tokens to stake

<img src="./images/miner_stake_1.png"  width="800" height="300">

2. Ensure your metamask wallet is on the same network as your selected network, and approve the transactions

<img src="./images/miner_stake_3.png"  width="800" height="300">

3. Once the previous step has completed, you can delegate to the miner address. For this use the miner address you have supplied to the miner node.

<img src="./images/miner_stake_2.png"  width="800" height="200">