// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

interface IAdapterOwner {
    struct AdapterConfig {
        // Minimum number of blocks a request must wait before being fulfilled.
        uint16 minimumRequestConfirmations;
        // Maximum gas limit for fulfillRandomness requests.
        uint32 maxGasLimit;
        // Reentrancy protection.
        bool reentrancyLock;
        // The fallback ARPA/ETH price to use when the feed is stale.
        int256 fallbackWeiPerUnitArpa;
        // stalenessSeconds is how long before we consider the feed price to be stale
        // and fallback to fallbackWeiPerUnitArpa.
        uint32 stalenessSeconds;
        // Gas to cover group payment after we calculate the payment.
        // We make it configurable in case those operations are repriced.
        uint32 gasAfterPaymentCalculation;
        // Gas except callback during fulfillment of randomness. Only used for estimating inflight cost.
        uint32 gasExceptCallback;
        // The assigned group is exclusive for fulfilling the task within this block window
        uint256 signatureTaskExclusiveWindow;
        // reward per signature for every participating node
        uint256 rewardPerSignature;
        // reward per signature for the committer
        uint256 committerRewardPerSignature;
    }

    struct FeeConfig {
        // Flat fee charged per fulfillment in millionths of arpa
        uint32 fulfillmentFlatFeeArpaPPMTier1;
        uint32 fulfillmentFlatFeeArpaPPMTier2;
        uint32 fulfillmentFlatFeeArpaPPMTier3;
        uint32 fulfillmentFlatFeeArpaPPMTier4;
        uint32 fulfillmentFlatFeeArpaPPMTier5;
        uint24 reqsForTier2;
        uint24 reqsForTier3;
        uint24 reqsForTier4;
        uint24 reqsForTier5;
    }

    /**
     * @notice Sets the configuration of the adapter
     * @param minimumRequestConfirmations global min for request confirmations
     * @param maxGasLimit global max for request gas limit
     * @param stalenessSeconds if the eth/arpa feed is more stale then this, use the fallback price
     * @param gasAfterPaymentCalculation gas used in doing accounting after completing the gas measurement
     * @param fallbackWeiPerUnitArpa fallback eth/arpa price in the case of a stale feed
     * @param signatureTaskExclusiveWindow window in which a signature task is exclusive to the assigned group
     * @param rewardPerSignature reward per signature for every participating node
     * @param committerRewardPerSignature reward per signature for the committer
     * @param feeConfig fee tier configuration
     */
    function setAdapterConfig(
        uint16 minimumRequestConfirmations,
        uint32 maxGasLimit,
        uint32 stalenessSeconds,
        uint32 gasAfterPaymentCalculation,
        uint32 gasExceptCallback,
        int256 fallbackWeiPerUnitArpa,
        uint256 signatureTaskExclusiveWindow,
        uint256 rewardPerSignature,
        uint256 committerRewardPerSignature,
        FeeConfig memory feeConfig
    ) external;
}
