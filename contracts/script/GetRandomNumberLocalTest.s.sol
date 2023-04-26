// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "./ArpaLocalTest.sol";
import "../src/interfaces/IAdapter.sol";
import "../src/user/examples/GetRandomNumberExample.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

contract GetRandomNumberLocalTestScript is Script {
    function setUp() public {}

    function run() external {
        GetRandomNumberExample getRandomNumberExample;
        Arpa arpa;
        IAdapter adapter;

        uint256 plentyOfArpaBalance = vm.envUint("PLENTY_OF_ARPA_BALANCE");
        address adapterAddress = vm.envAddress("ADAPTER_ADDRESS");
        address arpaAddress = vm.envAddress("ARPA_ADDRESS");
        uint256 userPrivateKey = vm.envUint("USER_PRIVATE_KEY");
        address userAddress = vm.envAddress("USER_ADDRESS");

        adapter = IAdapter(adapterAddress);
        arpa = Arpa(arpaAddress);

        vm.broadcast(userPrivateKey);
        getRandomNumberExample = new GetRandomNumberExample(
            adapterAddress
        );

        vm.broadcast(userPrivateKey);
        arpa.mint(userAddress, plentyOfArpaBalance);

        vm.broadcast(userPrivateKey);
        arpa.approve(address(adapter), plentyOfArpaBalance);

        vm.broadcast(userPrivateKey);
        uint64 subId = adapter.createSubscription();

        // have to set nonce manually or else the tx will fail
        vm.setNonce(userAddress, 8);

        vm.broadcast(userPrivateKey);
        adapter.fundSubscription(subId, plentyOfArpaBalance);

        vm.broadcast(userPrivateKey);
        adapter.addConsumer(subId, address(getRandomNumberExample));

        vm.broadcast(userPrivateKey);
        getRandomNumberExample.getRandomNumber();

        vm.broadcast(userPrivateKey);
        getRandomNumberExample.getRandomNumber();
    }
}
