// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../../src/user/examples/GetRandomNumberExample.sol";
import "../../src/user/examples/GetShuffledArrayExample.sol";
import "../../src/user/examples/RollDiceExample.sol";
import "../../src/user/examples/AdvancedGetShuffledArrayExample.sol";
import "../RandcastTestHelper.sol";

contract RandcastConsumerExampleTest is RandcastTestHelper {
    GetRandomNumberExample getRandomNumberExample;
    GetShuffledArrayExample getShuffledArrayExample;
    RollDiceExample rollDiceExample;
    AdvancedGetShuffledArrayExample advancedGetShuffledArrayExample;

    function setUp() public {
        skip(1000);
        changePrank(admin);
        arpa = new ERC20("arpa token", "ARPA");
        oracle = new MockArpaEthOracle();
        controller = new Controller(address(arpa), address(oracle));
        getRandomNumberExample = new GetRandomNumberExample(
            address(controller)
        );
        rollDiceExample = new RollDiceExample(address(controller));
        getShuffledArrayExample = new GetShuffledArrayExample(
            address(controller)
        );
        advancedGetShuffledArrayExample = new AdvancedGetShuffledArrayExample(
            address(controller)
        );

        uint256 nodeStakingAmount = 50000;
        uint256 disqualifiedNodePenaltyAmount = 1000;
        uint256 defaultNumberOfCommitters = 3;
        uint256 defaultDkgPhaseDuration = 10;
        uint256 groupMaxCapacity = 10;
        uint256 idealNumberOfGroups = 5;
        uint256 pendingBlockAfterQuit = 100;
        uint256 dkgPostProcessReward = 100;
        controller.setControllerConfig(
            nodeStakingAmount,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            pendingBlockAfterQuit,
            dkgPostProcessReward
        );

        uint16 minimumRequestConfirmations = 3;
        uint32 maxGasLimit = 2000000;
        uint32 stalenessSeconds = 30;
        uint32 gasAfterPaymentCalculation = 30000;
        uint32 gasExceptCallback = 200000;
        int256 fallbackWeiPerUnitArpa = 1e12;
        uint256 signatureTaskExclusiveWindow = 10;
        uint256 rewardPerSignature = 50;
        uint256 committerRewardPerSignature = 100;
        controller.setAdapterConfig(
            minimumRequestConfirmations,
            maxGasLimit,
            stalenessSeconds,
            gasAfterPaymentCalculation,
            gasExceptCallback,
            fallbackWeiPerUnitArpa,
            signatureTaskExclusiveWindow,
            rewardPerSignature,
            committerRewardPerSignature,
            Adapter.FeeConfig(250000, 250000, 250000, 250000, 250000, 0, 0, 0, 0)
        );

        uint96 plentyOfArpaBalance = 1e6 * 1e18;
        deal(address(arpa), address(admin), 3 * plentyOfArpaBalance);
        arpa.approve(address(controller), 3 * plentyOfArpaBalance);
        prepareSubscription(address(getRandomNumberExample), plentyOfArpaBalance);
        prepareSubscription(address(rollDiceExample), plentyOfArpaBalance);
        prepareSubscription(address(getShuffledArrayExample), plentyOfArpaBalance);
        prepareAnAvailableGroup();
    }

    function testControllerAddress() public {
        emit log_address(address(controller));
        assertEq(getRandomNumberExample.controller(), address(controller));
        assertEq(rollDiceExample.controller(), address(controller));
        assertEq(getShuffledArrayExample.controller(), address(controller));
    }

    function testGetRandomNumber() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        uint32 times = 10;
        for (uint256 i = 0; i < times; i++) {
            bytes32 requestId = getRandomNumberExample.getRandomNumber();

            deal(node1, 1 * 1e18);
            changePrank(node1);
            fulfillRequest(requestId, i);

            changePrank(user);
            vm.roll(block.number + 1);
        }

        for (uint256 i = 0; i < getRandomNumberExample.lengthOfRandomnessResults(); i++) {
            emit log_uint(getRandomNumberExample.randomnessResults(i));
        }
        assertEq(getRandomNumberExample.lengthOfRandomnessResults(), times);
    }

    function testRollDice() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        uint32 bunch = 10;
        bytes32 requestId = rollDiceExample.rollDice(bunch);

        deal(node1, 1 * 1e18);
        changePrank(node1);
        fulfillRequest(requestId, 10);

        changePrank(user);

        for (uint256 i = 0; i < rollDiceExample.lengthOfDiceResults(); i++) {
            emit log_uint(rollDiceExample.diceResults(i));
            assertTrue(rollDiceExample.diceResults(i) > 0 && rollDiceExample.diceResults(i) <= 6);
        }
        assertEq(rollDiceExample.lengthOfDiceResults(), bunch);
    }

    function testGetShuffledArray() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        uint32 upper = 10;
        bytes32 requestId = getShuffledArrayExample.getShuffledArray(upper);

        deal(node1, 1 * 1e18);
        changePrank(node1);
        fulfillRequest(requestId, 11);

        changePrank(user);

        for (uint256 i = 0; i < upper; i++) {
            emit log_uint(getShuffledArrayExample.shuffleResults(i));
            assertTrue(
                getShuffledArrayExample.shuffleResults(i) >= 0 && getShuffledArrayExample.shuffleResults(i) < upper
            );
        }
        assertEq(getShuffledArrayExample.lengthOfShuffleResults(), upper);
    }

    function testAdvancedGetShuffledArray() public {
        changePrank(admin);
        uint96 plentyOfArpaBalance = 1e6 * 1e18;
        deal(address(arpa), address(admin), plentyOfArpaBalance);
        arpa.approve(address(controller), plentyOfArpaBalance);
        uint64 subId = prepareSubscription(address(advancedGetShuffledArrayExample), plentyOfArpaBalance);

        deal(user, 1 * 1e18);
        changePrank(user);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 0;
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
                emit log_uint(advancedGetShuffledArrayExample.shuffleResults(k, i));
                assertTrue(
                    advancedGetShuffledArrayExample.shuffleResults(k, i) >= 0
                        && advancedGetShuffledArrayExample.shuffleResults(k, i) < upper
                );
            }
        }
    }
}
