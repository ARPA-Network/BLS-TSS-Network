// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IController} from "../src/interfaces/IController.sol";

interface IControllerForTest is IController {
    // Give node staking reward penalty and freezeNode
    function slashNodeForTest(address nodeIdAddress, uint256 stakingPenalty, uint256 pendingBlock) external;

    function removeFromGroupForTest(uint256 memberIndex, uint256 groupIndex)
        external
        returns (bool needRebalance, bool needEmitGroupEvent);

    function rebalanceGroupForTest(uint256 groupAIndex, uint256 groupBIndex) external returns (bool);

    function minimumThresholdForTest(uint256 groupSize) external pure returns (uint256);

    function emitGroupEventForTest(uint256 groupIndex) external;

    function getMemberIndexByAddressForTest(uint256 groupIndex, address nodeIdAddress) external view returns (int256);

    function pickRandomIndexForTest(uint256 seed, uint256[] memory indices, uint256 count)
        external
        pure
        returns (uint256[] memory);

    function getNonDisqualifiedMajorityMembersForTest(
        address[] memory nodeAddresses,
        address[] memory disqualifiedNodes
    ) external pure returns (address[] memory);
}
