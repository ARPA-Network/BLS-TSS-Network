// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import "forge-std/console.sol";
import {ControllerOracle} from "../src/ControllerOracle.sol";

contract GetGroupFromL1AndUpdateL2Script is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    function run() external {'
        // get env variables
        address _commiterAddress = vm.envAddress("ADMIN_ADDRESS");
        address _controllerAddress = vm.envAddress("CONTROLLER_ADDRESS");
        address ControllerOracleAddress = vm.envAddress("OP_CONTROLLER_ORACLE_ADDRESS");
        string memory _l1RPC = vm.envString("L1_RPC");
        string memory _l2RPC = vm.envString("OP_RPC");

        // create and select fork for L1
        vm.createSelectFork(_l1RPC);
        // get group from L1
        ControllerOracle.Group memory group = ControllerOracle(_controllerAddress).getGroup(0);
        console.logUint(group.epoch);
        // create and select fork for L2
        vm.createSelectFork(_l2RPC);
        vm.startBroadcast(_deployerPrivateKey);
        group.epoch = group.epoch + 1; // this is needed, otherwise you get GroupObsolete error.
        // update L2 Group with L1 Group Info
        ControllerOracle(ControllerOracleAddress).updateGroup(_commiterAddress, group);
        group = ControllerOracle(ControllerOracleAddress).getGroup(0);
        console.logUint(group.epoch);
        vm.stopBroadcast();
    }
}
