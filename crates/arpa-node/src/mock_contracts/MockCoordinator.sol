// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

contract MockCoordinator {
    uint256 public dkgThreshold;
    bytes[] public dkgKeys;
    address[] public participants;
    bytes[] public sharesData;
    bytes[] public responsesData;
    bytes[] public justificationsData;
    int8 public currentPhase = 0;
    
    constructor(uint256 _threshold) {
        dkgThreshold = _threshold;
    }

    function setDkgKeys(uint256 _threshold, bytes[] memory _keys) external {
        dkgThreshold = _threshold;
        dkgKeys = _keys;
    }
    
    function setParticipants(address[] memory _participants) external {
        participants = _participants;
    }
    
    function setShares(bytes[] memory _shares) external {
        sharesData = _shares;
    }
    
    function setResponses(bytes[] memory _responses) external {
        responsesData = _responses;
    }
    
    function setJustifications(bytes[] memory _justifications) external {
        justificationsData = _justifications;
    }
    
    function setCurrentPhase(int8 _phase) external {
        currentPhase = _phase;
    }
    
    function getDkgKeys() external view returns (uint256, bytes[] memory) {
        return (dkgThreshold, dkgKeys);
    }
    
    function getParticipants() external view returns (address[] memory) {
        return participants;
    }
    
    function getShares() external view returns (bytes[] memory) {
        return sharesData;
    }
    
    function getResponses() external view returns (bytes[] memory) {
        return responsesData;
    }
    
    function getJustifications() external view returns (bytes[] memory) {
        return justificationsData;
    }
    
    function inPhase() external view returns (int8) {
        return currentPhase;
    }

    function setupTestScenario(
        uint256 _threshold,
        address[] memory _participants,
        bytes[] memory _keys,
        bytes[] memory _shares,
        bytes[] memory _responses,
        bytes[] memory _justifications
    ) external {
        dkgThreshold = _threshold;
        participants = _participants;
        dkgKeys = _keys;
        sharesData = _shares;
        responsesData = _responses;
        justificationsData = _justifications;
        currentPhase = 1; // 默认开始Phase 1
    }
    
    function clearAllData() external {
        delete dkgKeys;
        delete participants;
        delete sharesData;
        delete responsesData;
        delete justificationsData;
        currentPhase = 0;
        dkgThreshold = 0;
    }
    
    function getDataStats() external view returns (
        uint256 keysCount,
        uint256 participantsCount,
        uint256 sharesCount,
        uint256 responsesCount,
        uint256 justificationsCount
    ) {
        return (
            dkgKeys.length,
            participants.length,
            sharesData.length,
            responsesData.length,
            justificationsData.length
        );
    }
}