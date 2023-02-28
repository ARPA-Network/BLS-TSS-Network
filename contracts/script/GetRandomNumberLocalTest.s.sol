// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "./ArpaLocalTest.sol";
import "../src/Controller.sol";
import "../src/user/examples/GetRandomNumberExample.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

contract GetRandomNumberLocalTestScript is Script {
    function setUp() public {}

    function run() external {
        GetRandomNumberExample getRandomNumberExample;
        Arpa arpa;
        Controller controller;

        uint96 plentyOfArpaBalance = 1e6 * 1e18;
        address controllerAddress = 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0;
        address arpaAddress = 0x5FbDB2315678afecb367f032d93F642f64180aa3;
        uint256 userPrivateKey = 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d;
        address userAddress = 0x70997970C51812dc3A010C7d01b50e0d17dc79C8;

        controller = Controller(controllerAddress);
        arpa = Arpa(arpaAddress);

        vm.broadcast(userPrivateKey);
        getRandomNumberExample = new GetRandomNumberExample(
            controllerAddress
        );

        vm.broadcast(userPrivateKey);
        arpa.mint(userAddress, plentyOfArpaBalance);

        vm.broadcast(userPrivateKey);
        arpa.approve(address(controller), plentyOfArpaBalance);

        vm.broadcast(userPrivateKey);
        uint64 subId = controller.createSubscription();

        vm.broadcast(userPrivateKey);
        controller.fundSubscription(subId, plentyOfArpaBalance);

        vm.broadcast(userPrivateKey);
        controller.addConsumer(subId, address(getRandomNumberExample));

        vm.broadcast(userPrivateKey);
        getRandomNumberExample.getRandomNumber();

        vm.broadcast(userPrivateKey);
        getRandomNumberExample.getRandomNumber();
    }
}
