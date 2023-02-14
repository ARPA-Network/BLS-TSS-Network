// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../GeneralRandcastConsumerBase.sol";

contract GetRandomNumberExample is GeneralRandcastConsumerBase {
    /* requestId -> randomness */
    mapping(bytes32 => uint256) public randomResults;
    uint256[] public randomnessResults;

    constructor(address controller) BasicRandcastConsumerBase(controller) {}

    /**
     * Requests randomness
     */
    function getRandomNumber() external returns (bytes32) {
        bytes memory params;
        return requestRandomness(RequestType.Randomness, params);
    }

    /**
     * Callback function used by Randcast Controller
     */
    function fulfillRandomness(bytes32 requestId, uint256 randomness)
        internal
        override
    {
        randomResults[requestId] = randomness;
        randomnessResults.push(randomness);
    }

    function lengthOfRandomnessResults() public view returns (uint256) {
        return randomnessResults.length;
    }
}
