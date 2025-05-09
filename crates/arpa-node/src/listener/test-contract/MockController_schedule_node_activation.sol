contract MockController {
    address public nodeRegistryAddress;
    address public adapterAddress;

    constructor(address _nodeRegistryAddress) {
        nodeRegistryAddress = _nodeRegistryAddress;
        adapterAddress = address(0);
    }

    function getControllerConfig() external view returns (
        address nodeRegistryContractAddress,
        address adapterContractAddress,
        uint256 disqualifiedNodePenaltyAmount,
        uint256 defaultNumberOfCommitters,
        uint256 defaultDkgPhaseDuration,
        uint256 groupMaxCapacity,
        uint256 idealNumberOfGroups,
        uint256 dkgPostProcessReward
    ) {
        return (
            nodeRegistryAddress,
            adapterAddress,
            1000,  // disqualifiedNodePenaltyAmount
            5,     // defaultNumberOfCommitters
            100,   // defaultDkgPhaseDuration
            10,    // groupMaxCapacity
            3,     // idealNumberOfGroups
            100    // dkgPostProcessReward
        );
    }
}