// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../../src/user/examples/GetRandomNumberExample.sol";
import "../../src/user/examples/GetShuffledArrayExample.sol";
import "../../src/user/examples/RollDiceExample.sol";
import "../../src/user/examples/AdvancedGetShuffledArrayExample.sol";
import {
    IAdapter, Adapter, RandcastTestHelper, ERC20, ControllerForTest, AdapterForTest
} from "../RandcastTestHelper.sol";
import {IAdapterOwner} from "../../src/interfaces/IAdapterOwner.sol";

contract ConsumerRequestBalanceTest is RandcastTestHelper {
    GetRandomNumberExample getRandomNumberExample;
    GetShuffledArrayExample getShuffledArrayExample;
    RollDiceExample rollDiceExample;
    AdvancedGetShuffledArrayExample advancedGetShuffledArrayExample;

    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 10;
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;
    uint256 last_output = 2222222222222222;

    uint16 minimumRequestConfirmations = 3;
    uint32 maxGasLimit = 2000000;
    uint32 gasAfterPaymentCalculation = 80000;
    uint32 gasExceptCallback = 492000;
    uint256 signatureTaskExclusiveWindow = 10;
    uint256 rewardPerSignature = 50;
    uint256 committerRewardPerSignature = 100;

    uint16 flatFeePromotionGlobalPercentage = 100;
    bool isFlatFeePromotionEnabledPermanently = false;
    uint256 flatFeePromotionStartTimestamp = 0;
    uint256 flatFeePromotionEndTimestamp = 0;

    function setUp() public {
        skip(1000);
        vm.prank(admin);
        arpa = new ERC20("arpa token", "ARPA");

        address[] memory operators = new address[](5);
        operators[0] = node1;
        operators[1] = node2;
        operators[2] = node3;
        operators[3] = node4;
        operators[4] = node5;
        _prepareStakingContract(stakingDeployer, address(arpa), operators);

        vm.prank(admin);
        controller = new ControllerForTest(address(arpa), last_output);

        vm.prank(admin);
        adapter = new AdapterForTest(address(controller));

        vm.prank(user);
        getRandomNumberExample = new GetRandomNumberExample(
            address(adapter)
        );

        vm.prank(user);
        rollDiceExample = new RollDiceExample(address(adapter));

        vm.prank(user);
        getShuffledArrayExample = new GetShuffledArrayExample(
            address(adapter)
        );

        vm.prank(user);
        advancedGetShuffledArrayExample = new AdvancedGetShuffledArrayExample(
            address(adapter)
        );

        vm.prank(admin);
        controller.setControllerConfig(
            address(staking),
            address(adapter),
            operatorStakeAmount,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            pendingBlockAfterQuit,
            dkgPostProcessReward
        );

        vm.prank(admin);
        adapter.setAdapterConfig(
            minimumRequestConfirmations,
            maxGasLimit,
            gasAfterPaymentCalculation,
            gasExceptCallback,
            signatureTaskExclusiveWindow,
            rewardPerSignature,
            committerRewardPerSignature
        );

        vm.broadcast(admin);
        IAdapterOwner(address(adapter)).setFlatFeeConfig(
            IAdapterOwner.FeeConfig(250000, 250000, 250000, 250000, 250000, 0, 0, 0, 0),
            flatFeePromotionGlobalPercentage,
            isFlatFeePromotionEnabledPermanently,
            flatFeePromotionStartTimestamp,
            flatFeePromotionEndTimestamp
        );

        vm.prank(stakingDeployer);
        staking.setController(address(controller));

        prepareAnAvailableGroup();
    }

    function testAdapterAddress() public {
        emit log_address(address(adapter));
        assertEq(rollDiceExample.adapter(), address(adapter));
    }

    function testCannotRequestWithoutSubscription() public {
        deal(user, 1 * 1e18);
        vm.startPrank(user);

        uint32 bunch = 10;
        vm.expectRevert(Adapter.InvalidSubscription.selector);
        rollDiceExample.rollDice(bunch);
    }

    function testCannotRequestWithoutEnoughBalance() public {
        deal(user, 1 * 1e18);

        uint256 feweTHBalance = 1 * 1e10;

        _prepareSubscription(user, address(rollDiceExample), feweTHBalance);

        uint32 bunch = 10;
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(user);
        rollDiceExample.rollDice(bunch);
    }

    function testRequestGeneralExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(user, 1 * 1e18);

        // (1e18 arpa wei/arpa) (wei/gas * gas) / (wei/arpa) = arpa wei
        // paymentNoFee = (1e18 *
        //     weiPerUnitGas *
        //     (gasExceptCallback + callbackGasUsed) /
        //     uint256(weiPerUnitArpa);
        // callbackGasUsed = 501728 gas
        // WeiPerUnitArpa = 1e12 wei/arpa
        // weiPerUnitGas = 1e9 wei/gas
        // gasExceptCallback  = 563262 gas
        // flat fee = 250000 1e12 arpa wei
        // Actual: 904212000000000000000
        // Expected: 891728000000000000000
        uint256 expectedPayment = 1e9 * (563262 + 501728);

        uint256 plentyOfEthBalance = 1e16;
        uint64 subId = _prepareSubscription(user, address(rollDiceExample), plentyOfEthBalance);

        uint32 bunch = 10;
        vm.prank(user);
        bytes32 requestId = rollDiceExample.rollDice(bunch);

        deal(node1, 1 * 1e18);
        fulfillRequest(node1, requestId, 10);

        (uint256 afterBalance, uint256 inflightCost) = _getBalance(subId);

        // the upper limit of delta is 5%
        // maxPercentDelta is an 18 decimal fixed point number, where 1e18 == 100%
        assertApproxEqRel(plentyOfEthBalance - afterBalance, expectedPayment, 1e18 / 20);
        // inflight cost should be 0 after fulfillment
        assertEq(inflightCost, 0);

        for (uint256 i = 0; i < rollDiceExample.lengthOfDiceResults(); i++) {
            assertTrue(rollDiceExample.diceResults(i) > 0 && rollDiceExample.diceResults(i) <= 6);
        }
        assertEq(rollDiceExample.lengthOfDiceResults(), bunch);
    }

    function testCannotRequestWithTooMuchInflightCost() public {
        deal(user, 1 * 1e18);

        // give the balance just enough for one request
        // give more than 3 times actual payment since we estimate 3 times max gas fee
        uint256 someEthBalance = 11 * 3 * 1e14;
        _prepareSubscription(user, address(rollDiceExample), someEthBalance);
        uint32 bunch = 10;
        vm.prank(user);
        rollDiceExample.rollDice(bunch);
        // now we have an inflight request, then try to request again in the next block
        vm.roll(block.number + 1);
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(user);
        rollDiceExample.rollDice(bunch);
    }

    function testRequestAdvancedExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(user, 1 * 1e18);

        uint256 plentyOfEthBalance = 1e15;
        uint64 subId = _prepareSubscription(user, address(advancedGetShuffledArrayExample), plentyOfEthBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 0;
        // just cover actual gasused
        uint256 callbackGasLimit = 350000;
        uint256 callbackMaxGasPrice = 1 * 1e9;

        vm.prank(user);
        bytes32 requestId = advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );

        deal(node1, 1 * 1e18);
        fulfillRequest(node1, requestId, 12);

        assertEq(advancedGetShuffledArrayExample.lengthOfShuffleResults(), 1);

        for (uint256 k = 0; k < advancedGetShuffledArrayExample.lengthOfShuffleResults(); k++) {
            for (uint256 i = 0; i < upper; i++) {
                assertTrue(
                    advancedGetShuffledArrayExample.shuffleResults(k, i) >= 0
                        && advancedGetShuffledArrayExample.shuffleResults(k, i) < upper
                );
            }
        }
    }

    function testCannotRequestAdvancedExampleWithTooHighCallbackGasLimitAndCallbackMaxGasFee() public {
        deal(user, 1 * 1e18);

        uint256 someEthBalance = 1e18;
        uint64 subId = _prepareSubscription(user, address(advancedGetShuffledArrayExample), someEthBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 0;
        uint256 callbackGasLimit = 2e6;
        uint256 callbackMaxGasPrice = 1e3 * 1e9;
        // payment = 2e6 * 1e3 * 1e9 = 2e18

        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(user);
        advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );
    }
}
