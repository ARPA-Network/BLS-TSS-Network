// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import {Arpa} from "./ArpaLocalTest.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";

contract StakeNodeLocalTestScript is Script {
    address internal _stakingAddress = vm.envAddress("STAKING_ADDRESS");
    address internal _arpaAddress = vm.envAddress("ARPA_ADDRESS");

    uint256 internal _operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");
    uint32 internal _nodePrivateKeyOffset = uint32(vm.envUint("NODE_PRIVATE_KEY_OFFSET"));

    Staking internal _staking;
    Arpa internal _arpa;

    function run() external {
        _arpa = Arpa(_arpaAddress);
        _staking = Staking(_stakingAddress);

        string memory keyName = string(abi.encodePacked("NODE_PRIVATE_KEY_", Strings.toString(_nodePrivateKeyOffset)));
        uint256 privateKey = vm.envUint(keyName);
        // address operator = vm.addr(privateKey);
        // string memory privateKey = vm.envString(keyName);
        address operator = vm.rememberKey(privateKey);
        vm.startBroadcast(operator);
        _stake(operator);
        vm.stopBroadcast();
    }

    function _stake(address sender) internal {
        // vm.broadcast(sender);
        _arpa.mint(sender, _operatorStakeAmount);

        _arpa.approve(address(_staking), _operatorStakeAmount);

        _staking.stake(_operatorStakeAmount);
    }
}
