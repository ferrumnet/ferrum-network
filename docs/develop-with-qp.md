# Develop using Quantum Portal

### Overview

Quantum Portal is a smart contract framework that enables interoperability and communication between different blockchains. Solidity developers can utilize Quantum Portal in their contracts to interact with contracts on other chains.

Quantum Portal allows contracts to execute methods on remote contracts located on different blockchains.It enables value transfers, method calls, and balance transfers between chains.

Developers can use Quantum Portal to build cross-chain functionalities and integrate with contracts on other chains.

### Key Components

- `QuantumPortalPoc`: An abstract contract that provides the main functionality for Quantum Portal.
  - It inherits from `TokenReceivable` and `PortalLedger` contracts.
  - `TokenReceivable` handles token transfers, and `PortalLedger` manages transaction registration and balance tracking.

### Usage

- Solidity developers can inherit from `QuantumPortalPoc` to incorporate Quantum Portal into their contracts.
- The `run` function allows executing a method on a remote contract without value transfer.
- The `runWithValue` function executes a remote method and pays a specified token amount to the remote contract.
- The `runWithdraw` function performs a remote withdraw, updating the user's balance for subsequent withdrawals.
- The `remoteTransfer` function transfers the remote balance of a token to another account within a mining context.
- The `withdraw` function enables users to withdraw their remote balance.
- The `msgSender` function retrieves information about the current context, including the source network, message sender, and beneficiary.


### Your First Contract

For the fist example lets build a simple ping-pong contract, the contract will send a ping message to a remote contract deployed on another network and the pong contract will receive the ping message and send a pong message back to the caller.

<img src="./images/ping_pong.png"  width="600" height="400">

## Pong Contract

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
import "quantum-portal-smart-contracts/contracts/quantumPortal/poc/IQuantumPortalPoc.sol";
import "quantum-portal-smart-contracts/contracts/quantumPortal/poc/IQuantumPortalFeeManager.sol";

/**
 * @title Pong
 * @dev A smart contract that handles pinging and ponging between contracts using Quantum Portal.
 */
contract Pong  {
    uint256 public CHAIN_ID;
    IQuantumPortalPoc public portal;
    mapping (address => uint) public pings;
    address public pingContract;

    constructor() {
        initialize();
    }

    /**
     * @dev Initializes the Pong contract.
     */
    function initialize() internal virtual {
        uint256 overrideChainID; // for test only. provide 0 outside a test
        address portal_address;
        portal = IQuantumPortalPoc(portal_address);
        CHAIN_ID = overrideChainID == 0 ? block.chainid : overrideChainID;
    }

    /**
     * @notice This function should be called by the QuantumPortal.
     * @dev Handles the ping event triggered by the QuantumPortal.
     */
    function pingRemote() external {
        // caller is QP
        (uint netId, address sourceMsgSender, address beneficiary) = portal.msgSender();
        // ensure the caller is the ping contract
        require(sourceMsgSender == pingContract, "Caller not expected!");
        pings[sourceMsgSender] += 1;
    }

    /**
     * @dev Sends a pong response to the recipient on a specific chain.
     * @param recipient The address of the recipient to send the pong response.
     * @param chainId The ID of the chain on which the pong response is sent.
     */
    function pong(address recipient, uint256 chainId) external {
        pings[recipient] -= 1;
        bytes memory method = abi.encodeWithSelector(Ping.remotePong.selector);
        // Call the QuantumPortal to run the specified method on the given chain and contract
        portal.run(
            uint64(chainId), pingContract, msg.sender, method);
    }

    /**
     * @dev Sets the address of the ping contract.
     * @param contractAddress The address of the ping contract.
     */
    function setPingContractAddress(address contractAddress) external {
        pingContract = contractAddress;
    }
}
```


## Ping Contract

```solidity
pragma solidity ^0.8.0;
import "quantum-portal-smart-contracts/contracts/quantumPortal/poc/IQuantumPortalPoc.sol";
import "quantum-portal-smart-contracts/contracts/quantumPortal/poc/IQuantumPortalFeeManager.sol";
/**
 * @title Ping
 * @dev A smart contract that handles pinging and ponging between contracts using Quantum Portal.
 */
contract Ping {
    IQuantumPortalPoc public portal;
    uint256 public MASTER_CHAIN_ID = 26000; // The FRM chain ID
    address public PongContract;
    mapping (address => uint) public pongs;

    constructor() {
        initialize();
    }

    /**
     * @dev Initializes the Ping contract.
     */
    function initialize() internal virtual {
        uint256 overrideChainID; // for test only. provide 0 outside a test
        address portal_address;
        portal = IQuantumPortalPoc(portal_address);
    }

    /**
     * @dev Initiates the ping event.
     */
    function ping() external {
        bytes memory method = abi.encodeWithSelector(Pong.pingRemote.selector);
        portal.run(
            0, uint64(MASTER_CHAIN_ID), PongContract, msg.sender, method);
    }

    /**
     * @dev Handles the pong event triggered by the QuantumPortal.
     * @param recipient The address of the recipient of the pong event.
     */
    function remotePong(address recipient) external {
        pongs[recipient] += 1;
    }
}
```

You can view the full example here : https://github.com/ferrumnet/quantum-portal-tutorial-code-and-examples