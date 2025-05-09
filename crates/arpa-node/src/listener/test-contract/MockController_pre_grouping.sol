pragma solidity ^0.8.0;

contract MockController {
    event DkgTask(
        uint256 indexed globalEpoch,
        uint256 indexed groupIndex,
        uint256 indexed groupEpoch,
        uint256 size,
        uint256 threshold,
        address[] members,
        uint256 assignmentBlockHeight,
        address coordinatorAddress
    );
    
    function emitDkgTaskEvent(
        uint256 globalEpoch,
        uint256 groupIndex,
        uint256 groupEpoch,
        uint256 size,
        uint256 threshold,
        address[] memory members,
        uint256 assignmentBlockHeight,
        address coordinatorAddress
    ) external {
        emit DkgTask(
            globalEpoch,
            groupIndex,
            groupEpoch,
            size,
            threshold,
            members,
            assignmentBlockHeight,
            coordinatorAddress
        );
    }
}