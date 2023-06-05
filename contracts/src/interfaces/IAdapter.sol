// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import {IRequestTypeBase} from "./IRequestTypeBase.sol";

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

    struct RequestDetail {
        uint64 subId;
        uint256 groupIndex;
        RequestType requestType;
        bytes params;
        address callbackContract;
        uint256 seed;
        uint16 requestConfirmations;
        uint256 callbackGasLimit;
        uint256 callbackMaxGasPrice;
        uint256 blockNum;
    }

    // controller transaction
    function nodeWithdrawETH(address recipient, uint256 ethAmount) external;

    // consumer contract transaction
    function requestRandomness(RandomnessRequestParams calldata params) external returns (bytes32);

    function fulfillRandomness(
        uint256 groupIndex,
        bytes32 requestId,
        uint256 signature,
        RequestDetail calldata requestDetail,
        PartialSignature[] calldata partialSignatures
    ) external;

    // user transaction
    function createSubscription() external returns (uint64);

    function addConsumer(uint64 subId, address consumer) external;

    function fundSubscription(uint64 subId) external payable;

    function setReferral(uint64 subId, uint64 referralSubId) external;

    function cancelSubscription(uint64 subId, address to) external;

    function removeConsumer(uint64 subId, address consumer) external;

    // view
    function getLastSubscription(address consumer) external view returns (uint64);

    function getSubscription(uint64 subId)
        external
        view
        returns (
            address owner,
            address[] memory consumers,
            uint256 balance,
            uint256 inflightCost,
            uint64 reqCount,
            uint64 freeRequestCount,
            uint64 referralSubId,
            uint64 reqCountInCurrentPeriod,
            uint256 lastRequestTimestamp
        );

    function getPendingRequestCommitment(bytes32 requestId) external view returns (bytes32);

    function getLastRandomness() external view returns (uint256);

    function getRandomnessCount() external view returns (uint256);

    /*
     * @notice Compute fee based on the request count
     * @param reqCount number of requests
     * @return feePPM fee in ARPA PPM
     */
    function getFeeTier(uint64 reqCount) external view returns (uint32);

    // Estimate the amount of gas used for fulfillment
    function estimatePaymentAmountInETH(
        uint256 callbackGasLimit,
        uint256 gasExceptCallback,
        uint32 fulfillmentFlatFeeEthPPM,
        uint256 weiPerUnitGas
    ) external view returns (uint256);
}
