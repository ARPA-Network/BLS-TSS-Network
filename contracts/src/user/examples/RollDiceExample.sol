// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../GeneralRandcastConsumerBase.sol";

contract RollDiceExample is GeneralRandcastConsumerBase {
    /* requestId -> randomness */
    mapping(bytes32 => uint256[]) public randomResults;
    uint256[] public diceResults;

    constructor(address controller) BasicRandcastConsumerBase(controller) {}

    /**
     * Requests randomness
     */
    function rollDice(uint32 bunch) external returns (bytes32) {
        bytes memory params = abi.encode(bunch);
        return requestRandomness(RequestType.RandomWords, params);
    }

    /**
     * Callback function used by Randcast Controller
     */
    function fulfillRandomWords(bytes32 requestId, uint256[] memory randomWords)
        internal
        override
    {
        randomResults[requestId] = randomWords;
        diceResults = new uint256[](randomWords.length);
        for (uint32 i = 0; i < randomWords.length; i++) {
            diceResults[i] = (randomWords[i] % 6) + 1;
        }
    }

    function lengthOfDiceResults() public view returns (uint256) {
        return diceResults.length;
    }
}
