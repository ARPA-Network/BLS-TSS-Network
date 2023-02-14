// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../BasicRandcastConsumerBase.sol";
import "../../utils/RandomnessHandler.sol";
import "../../utils/RequestIdBase.sol";

contract AdvancedGetShuffledArrayExample is
    RequestIdBase,
    BasicRandcastConsumerBase,
    RandomnessHandler
{
    mapping(bytes32 => uint256) public shuffledArrayUppers;
    uint256[][] public shuffleResults;

    constructor(address controller) BasicRandcastConsumerBase(controller) {}

    /**
     * Requests randomness
     */
    function getRandomNumberThenGenerateShuffledArray(
        uint256 shuffledArrayUpper,
        uint64 subId,
        uint256 seed,
        uint16 requestConfirmations,
        uint256 callbackGasLimit,
        uint256 callbackMaxGasPrice
    ) external returns (bytes32) {
        bytes memory params;

        uint256 rawSeed = makeRandcastInputSeed(seed, address(this), nonce);
        // This should be identical to controller generated requestId.
        bytes32 requestId = makeRequestId(rawSeed);
        shuffledArrayUppers[requestId] = shuffledArrayUpper;

        return
            rawRequestRandomness(
                RequestType.Randomness,
                params,
                subId,
                seed,
                requestConfirmations,
                callbackGasLimit,
                callbackMaxGasPrice
            );

        // These equals to following code(recommended):
        // bytes32 requestId = rawRequestRandomness(
        //    RequestType.Randomness,
        //    params,
        //    subId,
        //    seed,
        //    requestConfirmations,
        //    callbackGasLimit,
        //    callbackMaxGasPrice
        // );

        // shuffledArrayUppers[requestId] = shuffledArrayUpper;
    }

    /**
     * Callback function used by Randcast Controller
     */
    function fulfillRandomness(bytes32 requestId, uint256 randomness)
        internal
        override
    {
        shuffleResults.push(
            shuffle(shuffledArrayUppers[requestId], randomness)
        );
    }

    function lengthOfShuffleResults() public view returns (uint256) {
        return shuffleResults.length;
    }
}
