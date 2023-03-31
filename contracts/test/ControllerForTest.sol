// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "src/Controller.sol";

contract ControllerForTest is Controller {
    constructor(address arpa, address arpaEthFeed) Controller(arpa, arpaEthFeed) {}

    // Give node staking reward penalty and freezeNode
    function slashNodeForTest(address nodeIdAddress, uint256 stakingPenalty, uint256 pendingBlock, bool handleGroup)
        public
    {
        slashNode(nodeIdAddress, stakingPenalty, pendingBlock, handleGroup);
    }

    function removeFromGroupForTest(address nodeIdAddress, uint256 groupIndex, bool emitEventInstantly)
        public
        returns (bool)
    {
        return removeFromGroup(nodeIdAddress, groupIndex, emitEventInstantly);
    }

    function rebalanceGroupForTest(uint256 groupAIndex, uint256 groupBIndex) public returns (bool) {
        return rebalanceGroup(groupAIndex, groupBIndex);
    }

    function minimumThresholdForTest(uint256 groupSize) public pure returns (uint256) {
        return minimumThreshold(groupSize);
    }

    function emitGroupEventForTest(uint256 groupIndex) public {
        return emitGroupEvent(groupIndex);
    }

    function getMemberIndexByAddressForTest(uint256 groupIndex, address nodeIdAddress) public view returns (int256) {
        return getMemberIndexByAddress(groupIndex, nodeIdAddress);
    }

    function pickRandomIndexForTest(uint256 seed, uint256[] memory indices, uint256 count)
        public
        pure
        returns (uint256[] memory)
    {
        return pickRandomIndex(seed, indices, count);
    }

    function getNonDisqualifiedMajorityMembersForTest(
        address[] memory nodeAddresses,
        address[] memory disqualifiedNodes
    ) public pure returns (address[] memory) {
        return getNonDisqualifiedMajorityMembers(nodeAddresses, disqualifiedNodes);
    }
}
