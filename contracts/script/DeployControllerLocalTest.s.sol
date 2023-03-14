// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/interfaces/IAdapter.sol";
import "../src/Controller.sol";
import "./MockArpaEthOracle.sol";
import "./ArpaLocalTest.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

contract DeployControllerTestScript is Script {
    uint256 deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");
    function setUp() public {}

    function run() external {
        Controller controller;
        MockArpaEthOracle oracle;
        IERC20 arpa;

        vm.broadcast(deployerPrivateKey);
        arpa = new Arpa();

        vm.broadcast(deployerPrivateKey);
        oracle = new MockArpaEthOracle();

        vm.broadcast(deployerPrivateKey);
        controller = new Controller(address(arpa), address(oracle));
    }
}
