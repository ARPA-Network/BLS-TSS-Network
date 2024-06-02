// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

interface INodeRegistryOwner {
    /**
     * @notice Sets the configuration of the NodeRegistry
     * @param controllerContract The address of the controller contract
     * @param stakingContract The address of the staking contract
     * @param serviceManagerContract The address of the service manager contract
     * @param nativeNodeStakingAmount The amount of ARPA must staked by a node
     * @param eigenlayerNodeStakingAmount The amount of token must restaked by an eigenlayer node
     * @param pendingBlockAfterQuit The number of blocks a node must wait before joining a group after quitting
     */
    function setNodeRegistryConfig(
        address controllerContract,
        address stakingContract,
        address serviceManagerContract,
        uint256 nativeNodeStakingAmount,
        uint256 eigenlayerNodeStakingAmount,
        uint256 pendingBlockAfterQuit
    ) external;

    function initialize(address arpa) external;

    /**
     * @notice Dismiss a node from the registry forcefully
     * @param nodeIdAddress The address of the node
     * @param pendingBlock The number of blocks the node must wait before activating again
     */
    function dismissNode(address nodeIdAddress, uint256 pendingBlock) external;

    /**
     * @notice Set the asset account of the node
     * @param assetAccountAddresses The addresses of the asset accounts
     * @param nodeAddresses The addresses of the nodes
     */
    function setAssetAccount(address[] calldata assetAccountAddresses, address[] calldata nodeAddresses) external;
}
