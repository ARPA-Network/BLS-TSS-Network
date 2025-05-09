// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IRequestTypeBase {
    enum RequestType {
        Randomness,
        RandomWords,
        Shuffling
    }
}

contract MockAdapter is IRequestTypeBase {
    event RandomnessRequest(
        bytes32 indexed requestId,
        uint64 indexed subId,
        uint32 indexed groupIndex,
        RequestType requestType,
        bytes params,
        address sender,
        uint256 seed,
        uint16 requestConfirmations,
        uint32 callbackGasLimit,
        uint256 callbackMaxGasPrice,
        uint256 estimatedPayment
    );

    function emitRandomnessRequest(
        bytes32 requestId,
        uint64 subId,
        uint32 groupIndex,
        RequestType requestType,
        bytes calldata params,
        address sender,
        uint256 seed,
        uint16 requestConfirmations,
        uint32 callbackGasLimit,
        uint256 callbackMaxGasPrice,
        uint256 estimatedPayment
    ) external {
        emit RandomnessRequest(
            requestId,
            subId,
            groupIndex,
            requestType,
            params,
            sender,
            seed,
            requestConfirmations,
            callbackGasLimit,
            callbackMaxGasPrice,
            estimatedPayment
        );
    }
}