// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script} from "forge-std/Script.sol";
import {IAdapter} from "../src/interfaces/IAdapter.sol";
import {AdvancedGetShuffledArrayExampleTest} from "../src/user/examples/AdvancedGetShuffledArrayExampleTest.sol";

contract GetRandomNumberLocalAndvancedTestScript is Script {
    function run() external {
        AdvancedGetShuffledArrayExampleTest advancedGetShuffledArrayExampleTest;
        IAdapter adapter;
        address adapterAddress = vm.envAddress("ADAPTER_ADDRESS");
        uint256 userPrivateKey = vm.envUint("USER_PRIVATE_KEY");

        adapter = IAdapter(adapterAddress);

        vm.startBroadcast(userPrivateKey);
        advancedGetShuffledArrayExampleTest = new AdvancedGetShuffledArrayExampleTest(
            adapterAddress
        );

        //uint64 subId = adapter.createSubscription();

        //adapter.fundSubscription{value: plentyOfEthBalance}(subId);

        adapter.addConsumer(2, address(advancedGetShuffledArrayExampleTest));
    }
}

