// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {Controller} from "../src/Controller.sol";

contract ChangeDeploymentControllerConfig is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    address internal _controllerAddress = vm.envAddress("CONTROLLER_ADDRESS");
    address internal _nodeRegistryAddress = vm.envAddress("NODE_REGISTRY_ADDRESS");
    address internal _adapterAddress = vm.envAddress("ADAPTER_ADDRESS");
    uint256 internal _disqualifiedNodePenaltyAmount = vm.envUint("DISQUALIFIED_NODE_PENALTY_AMOUNT");
    uint256 internal _defaultNumberOfCommitters = vm.envUint("DEFAULT_NUMBER_OF_COMMITTERS");
    uint256 internal _defaultDkgPhaseDuration = vm.envUint("DEFAULT_DKG_PHASE_DURATION");
    uint256 internal _groupMaxCapacity = vm.envUint("GROUP_MAX_CAPACITY");
    uint256 internal _idealNumberOfGroups = vm.envUint("IDEAL_NUMBER_OF_GROUPS");
    uint256 internal _dkgPostProcessReward = vm.envUint("DKG_POST_PROCESS_REWARD");

    Controller internal _controller;

    function run() external {
        _controller = Controller(_controllerAddress);

        vm.broadcast(_deployerPrivateKey);
        _controller.setControllerConfig(
            _nodeRegistryAddress,
            _adapterAddress,
            _disqualifiedNodePenaltyAmount,
            _defaultNumberOfCommitters,
            _defaultDkgPhaseDuration,
            _groupMaxCapacity,
            _idealNumberOfGroups,
            _dkgPostProcessReward
        );
    }
}
