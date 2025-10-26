// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IControllerRelayer {
    function relayGroup(uint256 chainId, uint256 groupIndex) external;
}

contract MockControllerRelayer is IControllerRelayer {
    bool public shouldSucceed = true;
    string public failureMessage = "";
    
    event GroupRelayed(
        uint256 epoch, 
        uint256 indexed groupIndex, 
        uint256 indexed groupEpoch, 
        address indexed committer
    );
    
    constructor() {}
    
    function relayGroup(uint256 chainId, uint256 groupIndex) external override {
        if (!shouldSucceed) {
            revert(failureMessage);
        }
        
        emit GroupRelayed(
            1,              // mock epoch
            groupIndex,     // actual groupIndex
            1,              // mock groupEpoch
            msg.sender      // actual committer
        );
    }
    
    function setShouldSucceed(bool _shouldSucceed) external {
        shouldSucceed = _shouldSucceed;
    }
    
    function setFailureMessage(string memory _message) external {
        failureMessage = _message;
    }
}