// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {Controller} from "../src/Controller.sol";
import {ControllerRelayer} from "../src/ControllerRelayer.sol";
import {BaseChainMessenger} from "../src/BaseChainMessenger.sol";
import {IControllerOwner} from "../src/interfaces/IControllerOwner.sol";
import {Adapter} from "../src/Adapter.sol";
import {IAdapterOwner} from "../src/interfaces/IAdapterOwner.sol";
import {Arpa} from "./ArpaLocalTest.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {Staking} from "Staking-v0.1/Staking.sol";

// solhint-disable-next-line max-states-count
contract CreateAndSetBaseChainMessengerScript is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    address internal _baseControllerOracleAddress = vm.envAddress("OP_CONTROLLER_ORACLE_ADDRESS");
    address internal _opL1CrossDomainMessengerAddress = vm.envAddress("OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS");
    address internal _controllerRelayer = vm.envAddress("EXISTING_L1_CONTROLLER_RELAYER");
    uint256 internal _baseChainId = vm.envUint("OP_CHAIN_ID");

    function run() external {
        ControllerRelayer controllerRelayer;
        BaseChainMessenger baseChainMessenger;

        controllerRelayer = ControllerRelayer(_controllerRelayer);

        vm.broadcast(_deployerPrivateKey);
        baseChainMessenger =
            new BaseChainMessenger(_controllerRelayer, _baseControllerOracleAddress, _opL1CrossDomainMessengerAddress);

        vm.broadcast(_deployerPrivateKey);
        controllerRelayer.setChainMessenger(_baseChainId, address(baseChainMessenger));
    }
}
