// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GeneralRandcastConsumerBase, BasicRandcastConsumerBase} from "../GeneralRandcastConsumerBase.sol";
// solhint-disable-next-line no-global-import
import "src/user/RandcastSDK.sol" as RandcastSDK;
contract PickPropertyExample is GeneralRandcastConsumerBase {
    /* requestId -> randomness */
    mapping(bytes32 => uint256) public randomResults;
    mapping(uint256 => string) public propertyValue;
    uint256 public indexResult;
    event PropertyValueResult(string);
    
    constructor(address controller) BasicRandcastConsumerBase(controller) {
        propertyValue[0] = "fire";
        propertyValue[1] = "wind";
        propertyValue[2] = "water";
    }

    /**
     * Requests randomness
     */
    function getProperty() external returns (bytes32) {
        bytes memory params;
        return _requestRandomness(RequestType.Randomness, params);
    }

    /**
     * Callback function used by Randcast Adapter
     */
    // solhint-disable-next-line
    function _fulfillRandomness(bytes32 requestId, uint256 randomness) internal override {
        randomResults[requestId] = randomness;
        uint256 propertyIndex = RandcastSDK.roll(randomness, 3);
        indexResult = propertyIndex;
        emit PropertyValueResult(propertyValue[propertyIndex]);
    }
}
