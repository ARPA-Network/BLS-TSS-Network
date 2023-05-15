// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/Controller.sol";

contract ChangeDeploymentControllerConfig is Script {
    uint256 deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    uint256 disqualifiedNodePenaltyAmount = vm.envUint("DISQUALIFIED_NODE_PENALTY_AMOUNT");
    uint256 defaultNumberOfCommitters = vm.envUint("DEFAULT_NUMBER_OF_COMMITTERS");
    uint256 defaultDkgPhaseDuration = vm.envUint("DEFAULT_DKG_PHASE_DURATION");
    uint256 groupMaxCapacity = vm.envUint("GROUP_MAX_CAPACITY");
    uint256 idealNumberOfGroups = vm.envUint("IDEAL_NUMBER_OF_GROUPS");
    uint256 pendingBlockAfterQuit = vm.envUint("PENDING_BLOCK_AFTER_QUIT");
    uint256 dkgPostProcessReward = vm.envUint("DKG_POST_PROCESS_REWARD");
    uint256 last_output = vm.envUint("LAST_OUTPUT");

    address controllerAddress = vm.envAddress("CONTROLLER_ADDRESS");
    address stakingAddress = vm.envAddress("STAKING_ADDRESS");
    address adapterAddress = vm.envAddress("ADAPTER_ADDRESS");
    uint256 operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");

    Controller controller;

    function setUp() public {}

    function run() external {
        controller = Controller(controllerAddress);

        vm.broadcast(deployerPrivateKey);
        controller.setControllerConfig(
            stakingAddress,
            adapterAddress,
            operatorStakeAmount,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            pendingBlockAfterQuit,
            dkgPostProcessReward
        );
    }
}
