// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import {Arpa} from "./ArpaLocalTest.sol";

contract InitStakingLocalTestScript is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");
    uint256 internal _userPrivateKey = vm.envUint("USER_PRIVATE_KEY");

    address internal _stakingAddress = vm.envAddress("STAKING_ADDRESS");
    address internal _arpaAddress = vm.envAddress("ARPA_ADDRESS");

    uint256 internal _rewardAmount = vm.envUint("REWARD_AMOUNT");
    uint256 internal _operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");

    address[] internal _operators;
    string internal _mnemonic = vm.envString("STAKING_NODES_MNEMONIC");
    uint32 internal _stakingNodesIndexOffset = uint32(vm.envUint("STAKING_NODES_INDEX_OFFSET"));
    uint32 internal _stakingNodesIndexLength = uint32(vm.envUint("STAKING_NODES_INDEX_LENGTH"));

    bool internal _local_test = vm.envBool("LOCAL_TEST");

    Staking internal _staking;
    Arpa internal _arpa;

    function run() external {
        _arpa = Arpa(_arpaAddress);
        _staking = Staking(_stakingAddress);

        if (_local_test == true) {
            vm.broadcast(_deployerPrivateKey); // ! commented out during testnet deployment.
            payable(vm.addr(_userPrivateKey)).transfer(100 ether); // ! commented out during testnet deployment.
        }

        // add operators
        for (uint32 i = _stakingNodesIndexOffset; i < _stakingNodesIndexOffset + _stakingNodesIndexLength; i++) {
            address operator = vm.rememberKey(vm.deriveKey(_mnemonic, i));
            _operators.push(operator);
            if (_local_test == true) {
                address payable toOperator = payable(operator); // ! commented out during testnet deployment.
                vm.broadcast(_deployerPrivateKey); // ! commented out during testnet deployment.
                toOperator.transfer(100 ether); // ! commented out during testnet deployment.
            }
        }

        vm.broadcast(_deployerPrivateKey);
        _staking.addOperators(_operators);

        // start the _staking pool
        vm.broadcast(_deployerPrivateKey);
        _arpa.mint(vm.addr(_deployerPrivateKey), _rewardAmount);

        vm.broadcast(_deployerPrivateKey);
        _arpa.approve(address(_staking), _rewardAmount);

        vm.broadcast(_deployerPrivateKey);
        _staking.start(_rewardAmount, 3 days); // ! should be parametrized

        // let a user stake to accumulate some rewards
        vm.broadcast(_userPrivateKey);
        _arpa.mint(vm.addr(_userPrivateKey), _operatorStakeAmount);

        vm.broadcast(_userPrivateKey);
        _arpa.approve(address(_staking), _operatorStakeAmount);

        vm.broadcast(_userPrivateKey);
        _staking.stake(_operatorStakeAmount);
    }
}
