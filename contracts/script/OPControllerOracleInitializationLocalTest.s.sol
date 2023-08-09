// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {ControllerOracle} from "../src/ControllerOracle.sol";
import {Adapter} from "../src/Adapter.sol";
import {IAdapterOwner} from "../src/interfaces/IAdapterOwner.sol";
import {Arpa} from "./ArpaLocalTest.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";

// solhint-disable-next-line max-states-count
contract OPControllerOracleInitializationLocalTestScript is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    uint256 internal _lastOutput = vm.envUint("LAST_OUTPUT");
    address internal _arpaAddress = vm.envAddress("OP_ARPA_ADDRESS");
    address internal _controllerOracleAddress = vm.envAddress("OP_CONTROLLER_ORACLE_ADDRESS");
    address internal _adapterAddress = vm.envAddress("OP_ADAPTER_ADDRESS");
    address internal _opChainMessengerAddress = vm.envAddress("OP_CHAIN_MESSENGER_ADDRESS");
    address internal _opL2CrossDomainMessengerAddress = vm.envAddress("OP_L2_CROSS_DOMAIN_MESSENGER_ADDRESS");

    function run() external {
        vm.broadcast(_deployerPrivateKey);
        ControllerOracle(_controllerOracleAddress).initialize(
            _arpaAddress, _opChainMessengerAddress, _opL2CrossDomainMessengerAddress, _adapterAddress, _lastOutput
        );
    }
}
