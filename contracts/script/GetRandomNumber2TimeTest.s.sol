// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {IAdapter} from "../src/interfaces/IAdapter.sol";
import {AdvancedGetShuffledArrayExample} from "Randcast-User-Contract/user/examples/AdvancedGetShuffledArrayExample.sol";

contract GetRandomNumber2TimeTestScript is Script {
        function run() external {
        AdvancedGetShuffledArrayExample _advancedGetShuffledArrayExample;
        IAdapter adapter;
        uint256 plentyOfEthBalance = 10000000000000000;
        address adapterAddress = vm.envAddress("ADAPTER_ADDRESS");
        uint256 userPrivateKey = vm.envUint("USER_PRIVATE_KEY");
        adapter = IAdapter(adapterAddress);
        vm.startBroadcast(userPrivateKey);
        _advancedGetShuffledArrayExample = new AdvancedGetShuffledArrayExample(
            adapterAddress
        );
        uint64 subId = adapter.createSubscription();
        adapter.fundSubscription{value: plentyOfEthBalance}(subId);
        adapter.addConsumer(subId, address(_advancedGetShuffledArrayExample));
        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 6;
        uint32 rdGasLimit = 2000000;
        uint256 rdMaxGasPrice = 1200000000;
        _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, rdGasLimit, rdMaxGasPrice
        );
        uint64 subId2 = adapter.createSubscription();
        adapter.fundSubscription{value: plentyOfEthBalance}(subId2);
        adapter.addConsumer(subId2, address(_advancedGetShuffledArrayExample));
        _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId2, seed, requestConfirmations, rdGasLimit, rdMaxGasPrice
        );
    }
}