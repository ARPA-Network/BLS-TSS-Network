// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script} from "forge-std/Script.sol";
import {Arpa} from "./ArpaLocalTest.sol";
import {IAdapter} from "../src/interfaces/IAdapter.sol";
import {GetRandomNumberExample} from "../src/user/examples/GetRandomNumberExample.sol";

contract GetRandomNumberLocalTestScript is Script {
    function run() external {
        GetRandomNumberExample getRandomNumberExample;
        Arpa arpa;
        IAdapter adapter;

        uint256 plentyOfEthBalance = vm.envUint("PLENTY_OF_ETH_BALANCE");
        address adapterAddress = vm.envAddress("ADAPTER_ADDRESS");
        address arpaAddress = vm.envAddress("ARPA_ADDRESS");
        uint256 userPrivateKey = vm.envUint("USER_PRIVATE_KEY");

        adapter = IAdapter(adapterAddress);
        arpa = Arpa(arpaAddress);

        vm.startBroadcast(userPrivateKey);
        getRandomNumberExample = new GetRandomNumberExample(
            adapterAddress
        );

        arpa.mint(vm.addr(userPrivateKey), plentyOfEthBalance);

        arpa.approve(address(adapter), plentyOfEthBalance);

        uint64 subId = adapter.createSubscription();

        adapter.fundSubscription{value: plentyOfEthBalance}(subId);

        adapter.addConsumer(subId, address(getRandomNumberExample));

        getRandomNumberExample.getRandomNumber();

        getRandomNumberExample.getRandomNumber();
    }
}
