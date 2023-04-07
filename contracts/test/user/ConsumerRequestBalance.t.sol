// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../../src/user/examples/GetRandomNumberExample.sol";
import "../../src/user/examples/GetShuffledArrayExample.sol";
import "../../src/user/examples/RollDiceExample.sol";
import "../../src/user/examples/AdvancedGetShuffledArrayExample.sol";
import "../RandcastTestHelper.sol";

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
    uint32 stalenessSeconds = 30;
    uint32 gasAfterPaymentCalculation = 80000;
    // TODO this is not confirmed
    uint32 gasExceptCallback = 575000;
    int256 fallbackWeiPerUnitArpa = 1e12;
    uint256 signatureTaskExclusiveWindow = 10;
    uint256 rewardPerSignature = 50;
    uint256 committerRewardPerSignature = 100;

    function setUp() public {
        skip(1000);
        vm.prank(admin);
        arpa = new ERC20("arpa token", "ARPA");
        vm.prank(admin);
        oracle = new MockArpaEthOracle();

        address[] memory operators = new address[](5);
        operators[0] = node1;
        operators[1] = node2;
        operators[2] = node3;
        operators[3] = node4;
        operators[4] = node5;
        prepareStakingContract(stakingDeployer, address(arpa), operators);

        vm.prank(admin);
        controller = new ControllerForTest(address(arpa), last_output);

        vm.prank(admin);
        adapter = new Adapter(address(controller), address(arpa), address(oracle));

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
            stalenessSeconds,
            gasAfterPaymentCalculation,
            gasExceptCallback,
            fallbackWeiPerUnitArpa,
            signatureTaskExclusiveWindow,
            rewardPerSignature,
            committerRewardPerSignature,
            IAdapterOwner.FeeConfig(250000, 250000, 250000, 250000, 250000, 0, 0, 0, 0)
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
        changePrank(user);

        uint32 bunch = 10;
        vm.expectRevert(Adapter.InvalidSubscription.selector);
        rollDiceExample.rollDice(bunch);
    }

    function testCannotRequestWithoutEnoughBalance() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        uint96 fewArpaBalance = 1 * 1e18;
        deal(address(arpa), address(user), fewArpaBalance);
        prepareSubscription(address(rollDiceExample), fewArpaBalance);

        uint32 bunch = 10;
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        rollDiceExample.rollDice(bunch);
    }

    function testRequestGeneralExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        // (1e18 arpa wei/arpa) (wei/gas * gas) / (wei/arpa) = arpa wei
        // paymentNoFee = (1e18 *
        //     weiPerUnitGas *
        //     (gasExceptCallback + callbackGasUsed) /
        //     uint256(weiPerUnitArpa);
        // callbackGasUsed = 501728 gas
        // WeiPerUnitArpa = 1e12 wei/arpa
        // weiPerUnitGas = 1e9 wei/gas
        // TODO gasExceptCallback  = 575000 gas
        // flat fee = 250000 1e12 arpa wei
        // Actual: 904212000000000000000
        // Expected: 891728000000000000000
        uint256 expectedPayment = (1e18 * 1e9 * (575000 + 501728)) / 1e12 + 250000 * 1e12;

        uint96 plentyOfArpaBalance = 1e6 * 1e18;
        deal(address(arpa), address(user), plentyOfArpaBalance);
        uint64 subId = prepareSubscription(address(rollDiceExample), plentyOfArpaBalance);

        uint32 bunch = 10;
        bytes32 requestId = rollDiceExample.rollDice(bunch);

        deal(node1, 1 * 1e18);
        changePrank(node1);
        fulfillRequest(requestId, 10);

        changePrank(user);
        (uint96 afterBalance, uint96 inflightCost) = getBalance(subId);

        // the upper limit of delta is 5%
        // maxPercentDelta is an 18 decimal fixed point number, where 1e18 == 100%
        assertApproxEqRel(plentyOfArpaBalance - afterBalance, expectedPayment, 1e18 / 20);
        // inflight cost should be 0 after fulfillment
        assertEq(inflightCost, 0);

        for (uint256 i = 0; i < rollDiceExample.lengthOfDiceResults(); i++) {
            assertTrue(rollDiceExample.diceResults(i) > 0 && rollDiceExample.diceResults(i) <= 6);
        }
        assertEq(rollDiceExample.lengthOfDiceResults(), bunch);
    }

    function testCannotRequestWithTooMuchInflightCost() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        // give the balance just enough for one request
        // give more than 3 times actual payment(1076242000000000000000) since we estimate 3 times max gas fee(3320434000000000000000)
        uint96 someArpaBalance = 1150 * 3 * 1e18;
        deal(address(arpa), address(user), someArpaBalance);
        prepareSubscription(address(rollDiceExample), someArpaBalance);
        uint32 bunch = 10;
        rollDiceExample.rollDice(bunch);
        // now we have an inflight request, then try to request again in the next block
        vm.roll(block.number + 1);
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        rollDiceExample.rollDice(bunch);
    }

    function testRequestAdvancedExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        uint96 plentyOfArpaBalance = 1e6 * 1e18;
        deal(address(arpa), address(user), plentyOfArpaBalance);
        uint64 subId = prepareSubscription(address(advancedGetShuffledArrayExample), plentyOfArpaBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 0;
        // just cover actual gasused
        uint256 callbackGasLimit = 350000;
        uint256 callbackMaxGasPrice = 1 * 1e9;

        bytes32 requestId = advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );

        deal(node1, 1 * 1e18);
        changePrank(node1);
        fulfillRequest(requestId, 12);

        changePrank(user);

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
        changePrank(user);

        uint96 plentyOfArpaBalance = 1e6 * 1e18;
        deal(address(arpa), address(user), plentyOfArpaBalance);
        uint64 subId = prepareSubscription(address(advancedGetShuffledArrayExample), plentyOfArpaBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 0;
        uint256 callbackGasLimit = 2e6;
        uint256 callbackMaxGasPrice = 1e3 * 1e9;

        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );
    }
}
