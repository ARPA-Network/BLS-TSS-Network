// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import "./ArpaLocalTest.sol";

contract StakeNodeLocalTestScript is Script {
    uint256 deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    address stakingAddress = vm.envAddress("STAKING_ADDRESS");
    address arpaAddress = vm.envAddress("ARPA_ADDRESS");
    address deployerAddress = vm.envAddress("ADMIN_ADDRESS");

    uint256 rewardAmount = vm.envUint("REWARD_AMOUNT");
    uint256 operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");

    address[] operators;
    string mnemonic = "test test test test test test test test test test test junk";
    uint32 stakingNodesIndexOffset = uint32(vm.envUint("STAKING_NODES_INDEX_OFFSET"));
    uint32 stakingNodesIndexLength = uint32(vm.envUint("STAKING_NODES_INDEX_LENGTH"));

    Staking staking;
    Arpa arpa;

    function setUp() public {}

    function run() external {
        arpa = Arpa(arpaAddress);
        staking = Staking(stakingAddress);

        // add operators
        for (uint32 i = stakingNodesIndexOffset; i < stakingNodesIndexOffset + stakingNodesIndexLength; i++) {
            address operator = vm.rememberKey(vm.deriveKey(mnemonic, i));
            operators.push(operator);

            address payable to_operator = payable(operator);
            vm.broadcast(deployerPrivateKey);
            to_operator.transfer(1 ether);
        }

        vm.broadcast(deployerPrivateKey);
        staking.addOperators(operators);

        // start the staking pool
        vm.broadcast(deployerPrivateKey);
        arpa.mint(deployerAddress, rewardAmount);

        vm.broadcast(deployerPrivateKey);
        arpa.approve(address(staking), rewardAmount);

        vm.broadcast(deployerPrivateKey);
        staking.start(rewardAmount, 30 days);

        // let a user stake to accumulate some rewards
        // have to set nonce manually or else the tx will fail
        vm.setNonce(deployerAddress, 12 + stakingNodesIndexLength);
        stake(deployerAddress);

        for (uint256 i = 0; i < operators.length; i++) {
            stake(operators[i]);
        }
    }

    function stake(address sender) internal {
        vm.broadcast(sender);
        arpa.mint(sender, operatorStakeAmount);

        vm.broadcast(sender);
        arpa.approve(address(staking), operatorStakeAmount);

        vm.broadcast(sender);
        staking.stake(operatorStakeAmount);
    }
}
