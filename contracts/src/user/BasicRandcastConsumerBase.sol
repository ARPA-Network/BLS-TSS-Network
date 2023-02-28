// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../interfaces/IAdapter.sol";

/**
 * @notice Interface for contracts using VRF randomness.
 * @notice Extends this and overrides particular fulfill callback function to use randomness safely.
 */
abstract contract BasicRandcastConsumerBase is IRequestTypeBase {
    address public immutable controller;
    // Nonce on the user's side(count from 1) for generating real requestId,
    // which should be identical to the nonce on controller's side, or it will be pointless.
    uint256 public nonce = 1;
    // Ignore fulfilling from controller check during fee estimation.
    bool isEstimatingCallbackGasLimit;

    modifier calculateCallbackGasLimit() {
        isEstimatingCallbackGasLimit = true;
        _;
        isEstimatingCallbackGasLimit = false;
    }

    constructor(address _controller) {
        controller = _controller;
    }

    function fulfillRandomness(bytes32 requestId, uint256 randomness) internal virtual {}

    function fulfillRandomWords(bytes32 requestId, uint256[] memory randomWords) internal virtual {}

    function fulfillShuffledArray(bytes32 requestId, uint256[] memory shuffledArray) internal virtual {}

    function rawRequestRandomness(
        RequestType requestType,
        bytes memory params,
        uint64 subId,
        uint256 seed,
        uint16 requestConfirmations,
        uint256 callbackGasLimit,
        uint256 callbackMaxGasPrice
    ) internal returns (bytes32) {
        nonce = nonce + 1;

        IAdapter.RequestRandomnessParams memory p = IAdapter.RequestRandomnessParams(
            requestType, params, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );

        return IAdapter(controller).requestRandomness(p);
    }

    function rawFulfillRandomness(bytes32 requestId, uint256 randomness) external {
        require(isEstimatingCallbackGasLimit || msg.sender == controller, "Only controller can fulfill");
        fulfillRandomness(requestId, randomness);
    }

    function rawFulfillRandomWords(bytes32 requestId, uint256[] memory randomWords) external {
        require(isEstimatingCallbackGasLimit || msg.sender == controller, "Only controller can fulfill");
        fulfillRandomWords(requestId, randomWords);
    }

    function rawFulfillShuffledArray(bytes32 requestId, uint256[] memory shuffledArray) external {
        require(isEstimatingCallbackGasLimit || msg.sender == controller, "Only controller can fulfill");
        fulfillShuffledArray(requestId, shuffledArray);
    }
}
