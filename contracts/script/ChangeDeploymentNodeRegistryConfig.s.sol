// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {NodeRegistry} from "../src/NodeRegistry.sol";

contract ChangeDeploymentNodeRegistryConfig is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    address internal _nodeRegistryAddress = vm.envAddress("NODE_REGISTRY_ADDRESS");
    address internal _controllerAddress = vm.envAddress("CONTROLLER_ADDRESS");
    address internal _stakingAddress = vm.envAddress("STAKING_ADDRESS");
    uint256 internal _operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");
    uint256 internal _pendingBlockAfterQuit = vm.envUint("PENDING_BLOCK_AFTER_QUIT");

    NodeRegistry internal _nodeRegistry;

    function run() external {
        _nodeRegistry = NodeRegistry(_nodeRegistryAddress);

        vm.broadcast(_deployerPrivateKey);
        _nodeRegistry.setNodeRegistryConfig(
            _controllerAddress, _stakingAddress, _operatorStakeAmount, _pendingBlockAfterQuit
        );
    }
}
