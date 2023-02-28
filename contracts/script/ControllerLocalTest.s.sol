// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/interfaces/IAdapter.sol";
import "../src/Controller.sol";
import "./MockArpaEthOracle.sol";
import "./ArpaLocalTest.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

contract ControllerLocalTestScript is Script {
    uint256 deployerPrivateKey = 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;
    uint16 minimumRequestConfirmations = 3;
    uint32 maxGasLimit = 2000000;
    uint32 stalenessSeconds = 30;
    uint32 gasAfterPaymentCalculation = 30000;
    uint32 gasExceptCallback = 200000;
    int256 fallbackWeiPerUnitArpa = 1e12;

    uint32 fulfillmentFlatFeeArpaPPMTier1 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier2 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier3 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier4 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier5 = 250000;
    uint24 reqsForTier2 = 0;
    uint24 reqsForTier3 = 0;
    uint24 reqsForTier4 = 0;
    uint24 reqsForTier5 = 0;

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

        vm.broadcast(deployerPrivateKey);
        controller.setConfig(
            minimumRequestConfirmations,
            maxGasLimit,
            stalenessSeconds,
            gasAfterPaymentCalculation,
            gasExceptCallback,
            fallbackWeiPerUnitArpa,
            Adapter.FeeConfig(
                fulfillmentFlatFeeArpaPPMTier1,
                fulfillmentFlatFeeArpaPPMTier2,
                fulfillmentFlatFeeArpaPPMTier3,
                fulfillmentFlatFeeArpaPPMTier4,
                fulfillmentFlatFeeArpaPPMTier5,
                reqsForTier2,
                reqsForTier3,
                reqsForTier4,
                reqsForTier5
            )
        );
    }
}
