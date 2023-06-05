// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Controller, GroupLib} from "../src/Controller.sol";
// solhint-disable-next-line no-global-import
import "../src/utils/Utils.sol" as Utils;

contract ControllerForTest is Controller {
    using GroupLib for GroupLib.GroupData;

    constructor(address arpa, uint256 lastOutput) {
        initialize(arpa, lastOutput);
    }

    // Give node staking reward penalty and freezeNode
    function slashNodeForTest(address nodeIdAddress, uint256 stakingPenalty, uint256 pendingBlock) public {
        _slashNode(nodeIdAddress, stakingPenalty, pendingBlock);
    }

    function removeFromGroupForTest(uint256 memberIndex, uint256 groupIndex)
        public
        returns (bool needRebalance, bool needEmitGroupEvent)
    {
        return _groupData.removeFromGroup(memberIndex, groupIndex);
    }

    function rebalanceGroupForTest(uint256 groupAIndex, uint256 groupBIndex) public returns (bool) {
        return _groupData.rebalanceGroup(groupAIndex, groupBIndex, this.getLastOutput());
    }

    function minimumThresholdForTest(uint256 groupSize) public pure returns (uint256) {
        return Utils.minimumThreshold(groupSize);
    }

    function emitGroupEventForTest(uint256 groupIndex) public {
        return _emitGroupEvent(groupIndex);
    }

    function getMemberIndexByAddressForTest(uint256 groupIndex, address nodeIdAddress) public view returns (int256) {
        return _groupData.getMemberIndexByAddress(groupIndex, nodeIdAddress);
    }

    function pickRandomIndexForTest(uint256 seed, uint256[] memory indices, uint256 count)
        public
        pure
        returns (uint256[] memory)
    {
        return Utils.pickRandomIndex(seed, indices, count);
    }

    function getNonDisqualifiedMajorityMembersForTest(
        address[] memory nodeAddresses,
        address[] memory disqualifiedNodes
    ) public pure returns (address[] memory) {
        return Utils.getNonDisqualifiedMajorityMembers(nodeAddresses, disqualifiedNodes);
    }
}
