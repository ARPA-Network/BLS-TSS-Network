// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

interface IControllerOwner {
    /**
     * @notice Sets the configuration of the controller
     * @param nodeRegistryContract The address of the staking contract
     * @param adapterContract The address of the adapter contract
     * @param disqualifiedNodePenaltyAmount The amount of ARPA will be slashed from a node if it is disqualified
     * @param defaultNumberOfCommitters The default number of committers for a DKG
     * @param defaultDkgPhaseDuration The default duration(block number) of a DKG phase
     * @param groupMaxCapacity The maximum number of nodes in a group
     * @param idealNumberOfGroups The ideal number of groups
     * @param dkgPostProcessReward The amount of ARPA will be rewarded to the node after dkgPostProcess is completed
     */
    function setControllerConfig(
        address nodeRegistryContract,
        address adapterContract,
        uint256 disqualifiedNodePenaltyAmount,
        uint256 defaultNumberOfCommitters,
        uint256 defaultDkgPhaseDuration,
        uint256 groupMaxCapacity,
        uint256 idealNumberOfGroups,
        uint256 dkgPostProcessReward
    ) external;

    function initialize(uint256 lastOutput) external;
}
