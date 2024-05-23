// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {stdJson} from "forge-std/StdJson.sol";
import {Script} from "forge-std/Script.sol";

abstract contract Deployer is Script {
    string internal _root = vm.projectRoot();
    string internal _addressesPath = string.concat(_root, "/", vm.envString("DEPLOYMENT_ADDRESSES_PATH"));

    enum Network {
        L1,
        L2
    }

    function _checkDeploymentAddressesFile() internal {
        if (!vm.isFile(_addressesPath)) {
            string memory obj = "";
            vm.serializeString(obj, "L1", "");
            obj = vm.serializeString(obj, "L2", "");
            vm.writeJson(obj, _addressesPath);
        }
    }

    function _addDeploymentAddress(Network network, string memory name, address addr) internal {
        if (network == Network.L1) {
            vm.writeJson(stdJson.serialize("L1", name, addr), _addressesPath, ".L1");
        } else {
            vm.writeJson(stdJson.serialize("L2", name, addr), _addressesPath, ".L2");
        }
    }
}
