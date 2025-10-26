// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IRequestTypeBase {
    enum RequestType {
        Randomness,
        RandomWords,
        Shuffling
    }
}

interface IAdapter {
    function getPendingRequestCommitment(bytes32 requestId) external view returns (bytes32);
}

contract MockAdapter is IRequestTypeBase, IAdapter {
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

    event RandomnessRequestResult(
        bytes32 indexed requestId,
        uint32 indexed groupIndex,
        address committer,
        address[] participantMembers,
        uint256 randomness,
        uint256 payment,
        uint256 flatFee,
        bool success
    );

    mapping(bytes32 => bytes32) public _requestCommitments;
    mapping(bytes32 => bool) public shouldRevert;
    mapping(bytes32 => bool) public shouldRevertWithCustomError;
    
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
    
    function getPendingRequestCommitment(bytes32 requestId) public view override(IAdapter) returns (bytes32) {
        return _requestCommitments[requestId];
    }
    
    function setRequestCommitment(bytes32 requestId, bytes32 commitment) public {
        _requestCommitments[requestId] = commitment;
    }

    function setShouldRevert(bytes32 requestId, bool _shouldRevert) public {
        shouldRevert[requestId] = _shouldRevert;
    }

    function setShouldRevertWithCustomError(bytes32 requestId, bool _shouldRevertWithCustomError) public {
        shouldRevertWithCustomError[requestId] = _shouldRevertWithCustomError;
    }

    function fulfillRandomness(
        uint32 groupIndex,
        bytes32 requestId,
        uint256 signature,
        bytes calldata /* requestDetail */,
        bytes calldata /* partialSignatures */
    ) public {
        if (shouldRevertWithCustomError[requestId]) {
            revert("CustomTestError");
        }
        
        if (shouldRevert[requestId]) {
            revert("TestRevert");
        }

        delete _requestCommitments[requestId];

        address[] memory participantMembers = new address[](1);
        participantMembers[0] = msg.sender;
        
        emit RandomnessRequestResult(
            requestId,
            groupIndex,
            msg.sender,
            participantMembers,
            uint256(keccak256(abi.encode(signature))), 
            1000, 
            100,  
            true  
        );
    }
}