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

    event RandomnessRequest(
        uint256 indexed groupIndex,
        bytes32 requestId,
        address sender,
        uint64 subId,
        uint256 seed,
        uint16 requestConfirmations,
        uint256 callbackGasLimit,
        uint256 callbackMaxGasPrice
    );

    event RandomnessRequestResult(bytes32 requestId, uint256 output, uint256 payment, bool success);

    function requestRandomness(RandomnessRequestParams memory p) external returns (bytes32);

    function fulfillRandomness(
        uint256 groupIndex,
        bytes32 requestId,
        uint256 signature,
        PartialSignature[] calldata partialSignatures
    ) external;

    function getLastSubscription(address consumer) external view returns (uint64);

    function getSubscription(uint64 subId)
        external
        view
        returns (uint96 balance, uint96 inflightCost, uint64 reqCount, address owner, address[] memory consumers);
}
