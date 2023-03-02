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

        uint256 plentyOfArpaBalance = vm.envUint("PLENTY_OF_ARPA_BALANCE");
        address controllerAddress = vm.envAddress("CONTROLLER_ADDRESS");
        address arpaAddress = vm.envAddress("ARPA_ADDRESS");
        uint256 userPrivateKey = vm.envUint("USER_PRIVATE_KEY");
        address userAddress = vm.envAddress("USER_ADDRESS");

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
