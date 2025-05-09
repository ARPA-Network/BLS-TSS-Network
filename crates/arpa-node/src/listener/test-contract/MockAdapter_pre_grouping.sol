// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IAdapter {
    function getPendingRequestCommitment(bytes32 requestId) external view returns (bytes32);
}

contract MockAdapter is IAdapter {
    mapping(bytes32 => bytes32) public _requestCommitments;
    
    function getPendingRequestCommitment(bytes32 requestId) public view override(IAdapter) returns (bytes32) {
        return _requestCommitments[requestId];
    }
    
    function setRequestCommitment(bytes32 requestId, bytes32 commitment) public {
        _requestCommitments[requestId] = commitment;
    }
}