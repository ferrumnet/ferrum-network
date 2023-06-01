# Deploy solidity contracts

Ferrum network integrates the `frontier` libraries from substrate. This means you can deploy any solidity/evm compatible smart contract to the ferrum testnet.

## Deploy solidity contract to testnet

Deploying a solidity contract to testnet, is similar to deployment to any evm chain, this can be done in numerous ways, we will highlight two examples of deployment below
### Using Hardhat

To deploy to ferrum testnet using Hardhat, use the following config below

```json
    ferrum_testnet: {
      chainId: 26100,
      url: "http://testnet.dev.svcs.ferrumnetwork.io:8545",
      allowUnlimitedContractSize: true,
      gas: 10000000,
    },
```