// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../interfaces/IAdapter.sol";

/**
 * @notice Interface for contracts using VRF randomness.
 * @notice Extends this and overrides particular fulfill callback function to use randomness safely.
 */
abstract contract BasicRandcastConsumerBase is IRequestTypeBase {
    address public immutable adapter;
    // Nonce on the user's side(count from 1) for generating real requestId,
    // which should be identical to the nonce on adapter's side, or it will be pointless.
    uint256 public nonce = 1;
    // Ignore fulfilling from adapter check during fee estimation.
    bool isEstimatingCallbackGasLimit;

    modifier calculateCallbackGasLimit() {
        isEstimatingCallbackGasLimit = true;
        _;
        isEstimatingCallbackGasLimit = false;
    }

    constructor(address _adapter) {
        adapter = _adapter;
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

        IAdapter.RandomnessRequestParams memory p = IAdapter.RandomnessRequestParams(
            requestType, params, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );

        return IAdapter(adapter).requestRandomness(p);
    }

    function rawFulfillRandomness(bytes32 requestId, uint256 randomness) external {
        require(isEstimatingCallbackGasLimit || msg.sender == adapter, "Only adapter can fulfill");
        fulfillRandomness(requestId, randomness);
    }

    function rawFulfillRandomWords(bytes32 requestId, uint256[] memory randomWords) external {
        require(isEstimatingCallbackGasLimit || msg.sender == adapter, "Only adapter can fulfill");
        fulfillRandomWords(requestId, randomWords);
    }

    function rawFulfillShuffledArray(bytes32 requestId, uint256[] memory shuffledArray) external {
        require(isEstimatingCallbackGasLimit || msg.sender == adapter, "Only adapter can fulfill");
        fulfillShuffledArray(requestId, shuffledArray);
    }
}
