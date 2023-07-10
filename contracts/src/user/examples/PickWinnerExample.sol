// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GeneralRandcastConsumerBase, BasicRandcastConsumerBase} from "../GeneralRandcastConsumerBase.sol";
// solhint-disable-next-line no-global-import
import "src/user/RandcastSDK.sol" as RandcastSDK;
contract PickWinnerExample is GeneralRandcastConsumerBase {
    /* requestId -> randomness */
    mapping(bytes32 => uint256) public randomResults;
    uint256 public indexResult;
    event WinnerResult(uint256);

    // solhint-disable-next-line no-empty-blocks
    constructor(address controller) BasicRandcastConsumerBase(controller) {

    }
    /**
     * Requests randomness
     */
    function getWinner() external returns (bytes32) {
        bytes memory params;
        return _requestRandomness(RequestType.Randomness, params);
    }

    /**
     * Callback function used by Randcast Adapter
     */
    // solhint-disable-next-line
    function _fulfillRandomness(bytes32 requestId, uint256 randomness) internal override {
        randomResults[requestId] = randomness;
        uint256 winnerIndex = RandcastSDK.roll(randomness, 2);
        indexResult = winnerIndex;
        emit WinnerResult(winnerIndex);
    }
}
