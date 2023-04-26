// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "./IRequestTypeBase.sol";

interface IAdapter is IRequestTypeBase {
    struct PartialSignature {
        uint256 index;
        uint256 partialSignature;
    }

    struct RandomnessRequestParams {
        RequestType requestType;
        bytes params;
        uint64 subId;
        uint256 seed;
        uint16 requestConfirmations;
        uint256 callbackGasLimit;
        uint256 callbackMaxGasPrice;
    }

    // TODO only record the hash of the callback params to save storage gas
    struct Callback {
        address callbackContract;
        RequestType requestType;
        bytes params;
        uint64 subId;
        uint256 seed;
        uint256 groupIndex;
        uint256 blockNum;
        uint16 requestConfirmations;
        uint256 callbackGasLimit;
        uint256 callbackMaxGasPrice;
    }

    function requestRandomness(RandomnessRequestParams memory p) external returns (bytes32);

    function fulfillRandomness(
        uint256 groupIndex,
        bytes32 requestId,
        uint256 signature,
        PartialSignature[] calldata partialSignatures
    ) external;

    function createSubscription() external returns (uint64);

    function addConsumer(uint64 subId, address consumer) external;

    function fundSubscription(uint64 subId, uint256 amount) external;

    function getLastSubscription(address consumer) external view returns (uint64);

    function getSubscription(uint64 subId)
        external
        view
        returns (uint96 balance, uint96 inflightCost, uint64 reqCount, address owner, address[] memory consumers);

    function getPendingRequest(bytes32 requestId) external view returns (Callback memory);

    function getLastRandomness() external view returns (uint256);

    function getRandomnessCount() external view returns (uint256);

    /*
     * @notice Compute fee based on the request count
     * @param reqCount number of requests
     * @return feePPM fee in ARPA PPM
     */
    function getFeeTier(uint64 reqCount) external view returns (uint32);

    // Estimate the amount of gas used for fulfillment
    function estimatePaymentAmount(
        uint256 callbackGasLimit,
        uint256 gasExceptCallback,
        uint32 fulfillmentFlatFeeArpaPPM,
        uint256 weiPerUnitGas
    ) external view returns (uint96);
}
