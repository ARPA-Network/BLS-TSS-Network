// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GeneralRandcastConsumerBase, BasicRandcastConsumerBase} from "../GeneralRandcastConsumerBase.sol";

contract GetShuffledArrayExample is GeneralRandcastConsumerBase {
    /* requestId -> randomness */
    mapping(bytes32 => uint256[]) public randomResults;
    uint256[] public shuffleResults;

    // solhint-disable-next-line no-empty-blocks
    constructor(address adapter) BasicRandcastConsumerBase(adapter) {}

    /**
     * Requests randomness
     */
    function getShuffledArray(uint32 upper) external returns (bytes32) {
        bytes memory params = abi.encode(upper);
        return _requestRandomness(RequestType.Shuffling, params);
    }

    /**
     * Callback function used by Randcast Adapter
     */
    function _fulfillShuffledArray(bytes32 requestId, uint256[] memory array) internal override {
        randomResults[requestId] = array;
        shuffleResults = array;
    }

    function lengthOfShuffleResults() public view returns (uint256) {
        return shuffleResults.length;
    }
}
