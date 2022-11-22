pragma solidity ^0.8.15;

interface ICoordinator {
    function PHASE_DURATION() external view returns (uint256);

    function THRESHOLD() external view returns (uint256);

    function getDkgKeys() external view returns (uint256, bytes[] memory);

    function getJustifications() external view returns (bytes[] memory);

    function getParticipants() external view returns (address[] memory);

    function getResponses() external view returns (bytes[] memory);

    function getShares() external view returns (bytes[] memory);

    function inPhase() external view returns (int8);

    function initialize(address[] memory nodes, bytes[] memory publicKeys)
        external;

    function justifications(address) external view returns (bytes memory);

    function participant_map(address) external view returns (bool);

    function participants(uint256) external view returns (address);

    function publish(bytes memory value) external;

    function responses(address) external view returns (bytes memory);

    function shares(address) external view returns (bytes memory);

    function startBlock() external view returns (uint256);
}
