// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script} from "forge-std/Script.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import {Arpa} from "./ArpaLocalTest.sol";

contract StakeOperatorScenarioTestScript is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");
    uint256 internal _userPrivateKey = vm.envUint("USER_PRIVATE_KEY");

    address internal _stakingAddress = vm.envAddress("STAKING_ADDRESS");
    address internal _arpaAddress = vm.envAddress("ARPA_ADDRESS");

    uint256 internal _rewardAmount = vm.envUint("REWARD_AMOUNT");
    uint256 internal _operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");

    address[] internal _operators;
    string internal _mnemonic = vm.envString("STAKING_NODES_MNEMONIC");
    uint32 internal _stakingNodesIndexOffset = uint32(vm.envUint("STAKING_NODES_INDEX_OFFSET"));
    uint32 internal _stakingNodesIndexLength = 10;
    bool internal _isStakeUser = vm.envBool("IS_STAKE_USER");

    Staking internal _staking;
    Arpa internal _arpa;

    function run() external {
        _arpa = Arpa(_arpaAddress);
        _staking = Staking(_stakingAddress);

        // add operators
        for (uint32 i = _stakingNodesIndexOffset; i < _stakingNodesIndexOffset + _stakingNodesIndexLength; i++) {
            address operator = vm.rememberKey(vm.deriveKey(_mnemonic, i));
            _operators.push(operator);

            address payable toOperator = payable(operator);
            vm.broadcast(_deployerPrivateKey);
            toOperator.transfer(1 ether);
        }

        vm.broadcast(_deployerPrivateKey);
        _staking.addOperators(_operators);

        // start the _staking pool
        vm.broadcast(_deployerPrivateKey);
        _arpa.mint(vm.addr(_deployerPrivateKey), _rewardAmount);

        vm.broadcast(_deployerPrivateKey);
        _arpa.approve(address(_staking), _rewardAmount);

        // let a user stake to accumulate some rewards
        if (_isStakeUser) {
            vm.broadcast(_deployerPrivateKey);
            _staking.start(_rewardAmount, 30 days);
            vm.rememberKey(_userPrivateKey);
            _stake(vm.addr(_userPrivateKey));
        }
    }

    function _stake(address sender) internal {
        vm.broadcast(sender);
        _arpa.mint(sender, _operatorStakeAmount);

        vm.broadcast(sender);
        _arpa.approve(address(_staking), _operatorStakeAmount);

        vm.broadcast(sender);
        _staking.stake(_operatorStakeAmount);
    }
}
