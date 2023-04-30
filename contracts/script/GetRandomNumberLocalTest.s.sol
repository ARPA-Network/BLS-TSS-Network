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

        vm.startBroadcast(userPrivateKey);
        getRandomNumberExample = new GetRandomNumberExample(
            adapterAddress
        );

        arpa.mint(userAddress, plentyOfArpaBalance);

        arpa.approve(address(adapter), plentyOfArpaBalance);

        uint64 subId = adapter.createSubscription();

        adapter.fundSubscription(subId, plentyOfArpaBalance);

        adapter.addConsumer(subId, address(getRandomNumberExample));

        getRandomNumberExample.getRandomNumber();

        getRandomNumberExample.getRandomNumber();
    }
}
