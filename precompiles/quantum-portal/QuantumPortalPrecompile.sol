// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @dev The finalizer precompile contract's address.
address constant QUANTUM_PORTAL_PRECOMPILE = 0x0000000000000000000000000000000000000812;

/**
 * @title QuantumPortalFinalizerPrecompile Interface
 * @dev Interface for managing finalizers in the Quantum Portal system.
 */
interface QuantumPortalFinalizerPrecompile {

    /**
     * @notice Register Finalizer
     * @dev Registers a finalizer for a specific chain ID.
     * @param chainId The unique identifier of the blockchain.
     * @param finalizer The address of the finalizer
     */
    function registerFinalizer(
        uint256 chainId,
        address finalizer
    ) external;

    /**
     * @notice Get Finalizers
     * @dev Retrieves the list of finalizers for a specific chain ID.
     * @param chainId The unique identifier of the blockchain.
     * @return The list of finalizers for the specified chain ID.
     */
    function getFinalizers(
        uint256 chainId
    ) external view returns (address[] memory);

    /**
     * @notice Add Finalizer
     * @dev Adds a finalizer for a specific chain ID at a specified index.
     * @param chainId The unique identifier of the blockchain.
     * @param index The index at which to add the finalizer.
     * @param finalizer The address of the finalizer to add.
     */
    function addFinalizer(
        uint256 chainId,
        uint256 index,
        address finalizer
    ) external;

    /**
     * @notice Remove Finalizer
     * @dev Removes a registered finalizer for a specific chain ID.
     * @param chainId The unique identifier of the blockchain.
     * @param finalizer The address of the finalizer
     */
    function removeFinalizer(
        uint256 chainId,
        address finalizer
    ) external;
}