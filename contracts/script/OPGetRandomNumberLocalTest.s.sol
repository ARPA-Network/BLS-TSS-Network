// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {IAdapter} from "../src/interfaces/IAdapter.sol";
import {GetRandomNumberExample} from "Randcast-User-Contract/user/examples/GetRandomNumberExample.sol";
import "forge-std/console.sol";

contract OPGetRandomNumberLocalTestScript is Script {
    function run() external {
        GetRandomNumberExample getRandomNumberExample;
        IAdapter adapter;

        uint256 plentyOfEthBalance = vm.envUint("L2_MIN_SUB_FUND_ETH_BAL");
        address adapterAddress = vm.envAddress("OP_ADAPTER_ADDRESS");
        uint256 userPrivateKey = vm.envUint("USER_PRIVATE_KEY");

        adapter = IAdapter(adapterAddress);

        vm.startBroadcast(userPrivateKey);
        getRandomNumberExample = new GetRandomNumberExample(
            adapterAddress
        );

        uint64 subId = adapter.createSubscription();

        adapter.fundSubscription{value: plentyOfEthBalance}(subId);

        adapter.addConsumer(subId, address(getRandomNumberExample));

        bool setGasPrice = vm.envBool("LOCAL_TEST");
        if (setGasPrice) {
            console.log(tx.gasprice);
            getRandomNumberExample.setCallbackGasConfig(0, 100000007);
        }
        getRandomNumberExample.getRandomNumber();

        // getRandomNumberExample.getRandomNumber();
    }
}
