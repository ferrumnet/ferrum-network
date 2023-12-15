// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;
import "./BtcPool.sol";

/**
 * @title BtcPoolsClient
 * @dev Smart contract to interact with the BtcPools contract through the BtcPools interface.
 */
contract BtcPoolsClient {
    BtcPools private btcPoolsContract;
    uint256 private constant MINIMUM_STAKE = 1000; // Minimum stake required in native tokens

    /**
     * @dev Constructor to set the address of the BtcPools contract.
     * @param _btcPoolsContractAddress The address of the BtcPools contract.
     */
    constructor(address _btcPoolsContractAddress) {
        btcPoolsContract = BtcPools(_btcPoolsContractAddress);
    }

    /**
     * @dev Submits a request to register a Bitcoin validator with BtcPools.
     * @param _submission The data to be submitted for Bitcoin validator registration.
     * @notice This function checks for a minimum stake of native tokens before calling
     *         the registerBtcValidator function of the BtcPools contract, initiating the registration process.
     * @dev Requirements:
     * - The caller must have a minimum stake of native tokens.
     * - The caller must be authorized to interact with the BtcPools contract.
     */
    function submitBtcValidatorRegistration(bytes32 _submission) external {

        // Call the registerBtcValidator function of the BtcPools contract
        btcPoolsContract.registerBtcValidator(_submission);

        // Additional logic or events can be added as needed.
    }
}
