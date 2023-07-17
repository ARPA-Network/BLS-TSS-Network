// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GeneralRandcastConsumerBase, BasicRandcastConsumerBase} from "../GeneralRandcastConsumerBase.sol";
// solhint-disable-next-line no-global-import
import "src/user/RandcastSDK.sol" as RandcastSDK;
contract PickRarityExample is GeneralRandcastConsumerBase {
    /* requestId -> randomness */
    mapping(bytes32 => uint256) public randomResults;
    mapping(uint256 => string) public rarityValue;
    uint256 public indexResult;
    uint256[] public rarityWeights = new uint256[](5);
    event RarityValueResult(string);
    constructor(address controller) BasicRandcastConsumerBase(controller) {
        rarityValue[0] = "SSSR";
        rarityValue[1] = "SSR";
        rarityValue[2] = "SR";
        rarityValue[3] = "R";
        rarityValue[4] = "C";

        rarityWeights[0] = 1;  // SSSR(1%)
        rarityWeights[1] = 4;  // SSR(4%)
        rarityWeights[2] = 10; // SR(10%)
        rarityWeights[3] = 20; // R(20%)
        rarityWeights[4] = 65; // C(65%)
    }

    /**
     * Requests randomness
     */
    function getRarity() external returns (bytes32) {
        bytes memory params;
        return _requestRandomness(RequestType.Randomness, params);
    }

    /**
     * Callback function used by Randcast Adapter
     */
    // solhint-disable-next-line
    function _fulfillRandomness(bytes32 requestId, uint256 randomness) internal override {
        randomResults[requestId] = randomness;
        uint256 rarityIndex = RandcastSDK.pickByWeights(randomness, rarityWeights);
        indexResult = rarityIndex;
        emit RarityValueResult(rarityValue[rarityIndex]);
    }
}
