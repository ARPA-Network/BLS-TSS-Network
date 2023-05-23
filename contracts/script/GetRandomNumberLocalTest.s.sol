// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "./ArpaLocalTest.sol";
import {IAdapter} from "../src/interfaces/IAdapter.sol";
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

        adapter = IAdapter(adapterAddress);
        arpa = Arpa(arpaAddress);

        vm.startBroadcast(userPrivateKey);
        getRandomNumberExample = new GetRandomNumberExample(
            adapterAddress
        );

        arpa.mint(vm.addr(userPrivateKey), plentyOfArpaBalance);

        arpa.approve(address(adapter), plentyOfArpaBalance);

        uint64 subId = adapter.createSubscription(IAdapter.TokenType.ARPA);

        adapter.fundSubscription(subId, plentyOfArpaBalance);

        adapter.addConsumer(subId, address(getRandomNumberExample));

        getRandomNumberExample.getRandomNumber();

        getRandomNumberExample.getRandomNumber();
    }
}
