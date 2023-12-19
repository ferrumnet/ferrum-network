// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract IntentRegistry {
    struct Intent {
        uint256 btcAmount;
        address btcAddress;
        address targetContract;
        bytes encodedCall;
        bool executed;
    }

    mapping(uint256 => Intent) public intents;
    uint256 public intentsCount;

    event IntentRegistered(uint256 indexed id, uint256 btcAmount, address btcAddress, address targetContract, bytes encodedCall);
    event IntentExecuted(uint256 indexed id);

    function registerIntent(uint256 _btcAmount, address _btcAddress, address _targetContract, bytes memory _encodedCall) external {
        intents[intentsCount] = Intent({
            btcAmount: _btcAmount,
            btcAddress: _btcAddress,
            targetContract: _targetContract,
            encodedCall: _encodedCall,
            executed: false
        });

        emit IntentRegistered(intentsCount, _btcAmount, _btcAddress, _targetContract, _encodedCall);

        intentsCount++;
    }

    function executeIntent(uint256 _intentId) external {
        require(_intentId < intentsCount, "Intent does not exist");
        require(!intents[_intentId].executed, "Intent already executed");

        // Execute the encoded call on the target contract
        (bool success, ) = intents[_intentId].targetContract.call(intents[_intentId].encodedCall);
        require(success, "Execution of the encoded call failed");

        // Mark the intent as executed
        intents[_intentId].executed = true;

        emit IntentExecuted(_intentId);
    }

    function getAllIntents() external view returns (Intent[] memory) {
        Intent[] memory allIntents = new Intent[](intentsCount);
        for (uint256 i = 0; i < intentsCount; i++) {
            allIntents[i] = intents[i];
        }
        return allIntents;
    }
}