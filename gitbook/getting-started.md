# Getting Started

## Table of Contents:

* [Connecting to Ferrum Network](#connecting-to-ferrum-network)
* [Requesting Testnet tokens](#requesting-testnet-tokens)
* [Transferring tokens](#transferring-tokens)
* [Verifying Transactions](#verifying-transactions)

## Connecting to ferrum network

### 1. Using PolkadotJS

Ferrum network is available at [testnet.dev.svcs.ferrumnetwork.io](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Ftestnet.dev.svcs.ferrumnetwork.io#/explorer)

You should be able to view the network like below : 

![Explorer](./images/explorer-example.png "metamask-example")

### 2. Using Metamask

To connect your metamask to Ferrum network, use the below configuration

```
Network Name : Ferrum Testnet

RPC URL : http://testnet.dev.svcs.ferrumnetwork.io:9933

ChainId : 26026

Currency : tFRM
```

The config should look like this :

<img src="./images/ferrum-metamask.png"  width="300" height="400">

## Requesting Testnet tokens

You can use the below faucet to request testnet tokens : 

https://testnet.faucet.ferrumnetwork.io/

## Transferring tokens

Ferrum network parachain supports all evm transactions, so transferring tFRM tokens should be like transferring any other erc20 token :

To transfer tFRM token on the Ferrum network, switch your metamask to `Ferrum Testnet`

<img src="./images/transfer_tokens_1.png"  width="250" height="400">

Enter the destination address and amount and confirm the transfer

<img src="./images/transfer_tokens_2.png"  width="250" height="400">


## Verifying transactions

All transactions (both substrate and evm transactions) can be seen on the ferrum explorer at [testnet.dev.svcs.ferrumnetwork.io](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Ftestnet.dev.svcs.ferrumnetwork.io#/explorer)

In the above case, if you navigate to the explorer, you should see the transfer events on the network tab like below :

<img src="./images/events_summary.png"  width="650" height="200">

If you click on any event, you should see the details of that event :

<img src="./images/events_details.png"  width="650" height="600">