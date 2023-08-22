// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import {Arpa} from "./ArpaLocalTest.sol";

contract StakeNodeLocalTestScript is Script {
    address internal _stakingAddress = vm.envAddress("STAKING_ADDRESS");
    address internal _arpaAddress = vm.envAddress("ARPA_ADDRESS");

    uint256 internal _operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");

    address[] internal _operators;
    string internal _mnemonic = vm.envString("STAKING_NODES_MNEMONIC");
    uint32 internal _stakingNodesIndexOffset = uint32(vm.envUint("STAKING_NODES_INDEX_OFFSET"));
    uint32 internal _stakingNodesIndexLength = uint32(vm.envUint("STAKING_NODES_INDEX_LENGTH"));

    Staking internal _staking;
    Arpa internal _arpa;

    function run() external {
        _arpa = Arpa(_arpaAddress);
        _staking = Staking(_stakingAddress);

        // get operators
        for (uint32 i = _stakingNodesIndexOffset; i < _stakingNodesIndexOffset + _stakingNodesIndexLength; i++) {
            address operator = vm.rememberKey(vm.deriveKey(_mnemonic, i));
            vm.startBroadcast(operator);
            _stake(operator);
            vm.stopBroadcast();
        }
    }

    function _stake(address sender) internal {
        // vm.broadcast(sender);
        _arpa.mint(sender, _operatorStakeAmount);

        _arpa.approve(address(_staking), _operatorStakeAmount);

        _staking.stake(_operatorStakeAmount);
    }
}
