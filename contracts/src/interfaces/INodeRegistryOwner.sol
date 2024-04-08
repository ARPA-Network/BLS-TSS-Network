// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

interface INodeRegistryOwner {
    /**
     * @notice Sets the configuration of the NodeRegistry
     * @param controllerContract The address of the controller contract
     * @param stakingContract The address of the staking contract
     * @param nodeStakingAmount The amount of ARPA must staked by a node
     * @param pendingBlockAfterQuit The number of blocks a node must wait before joining a group after quitting
     */
    function setNodeRegistryConfig(
        address controllerContract,
        address stakingContract,
        uint256 nodeStakingAmount,
        uint256 pendingBlockAfterQuit
    ) external;

    function initialize(address arpa, bool isEigenlayer) external;
}
