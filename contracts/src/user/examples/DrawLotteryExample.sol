// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GeneralRandcastConsumerBase, BasicRandcastConsumerBase} from "../GeneralRandcastConsumerBase.sol";
// solhint-disable-next-line no-global-import
import "src/user/RandcastSDK.sol" as RandcastSDK;
contract DrawLotteryExample is GeneralRandcastConsumerBase {
    /* requestId -> randomness */
    mapping(bytes32 => uint256[]) public randomResults;
    uint256[] public winnerResults;
    uint32 public ticketNumber;
    uint32 public winnerNumber;

    // solhint-disable-next-line no-empty-blocks
    constructor(address adapter) BasicRandcastConsumerBase(adapter) {}
    event LotteryTicketGenerated(uint256[] ticketResults);
    /**
     * Requests randomness
     */
    function getTickets(uint32 ticketNum, uint32 winnerNum) external returns (bytes32) {
        ticketNumber = ticketNum;
        winnerNumber = winnerNum;
        bytes memory params = abi.encode(ticketNumber);
        return _requestRandomness(RequestType.RandomWords, params);
    }

    /**
     * Callback function used by Randcast Adapter
     */
    function _fulfillRandomWords(bytes32 requestId, uint256[] memory randomWords) internal override {
        randomResults[requestId] = randomWords;
        emit LotteryTicketGenerated(randomWords);
        winnerResults = RandcastSDK.draw(randomWords[randomWords.length - 1], randomWords, winnerNumber);
    }

    function lengthOfWinnerResults() public view returns (uint256) {
        return winnerResults.length;
    }

    function getTicketResults(bytes32 requestId) public view returns (uint256[] memory) {
        return randomResults[requestId];
    }
}
