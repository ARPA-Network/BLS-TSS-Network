// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MockAdapter {
    enum RequestType {
        Randomness,
        RandomWords,
        Shuffling
    }
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

    mapping(bytes32 => bytes32) public _requestCommitments;

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

    function getPendingRequestCommitment(bytes32 requestId) public view returns (bytes32) {
        return _requestCommitments[requestId];
    }

    function setRequestCommitment(bytes32 requestId, bytes32 commitment) public {
        _requestCommitments[requestId] = commitment;
    }
}
