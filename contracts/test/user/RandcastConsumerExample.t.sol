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
    uint32 gasAfterPaymentCalculation = 30000;
    uint32 gasExceptRequestDetail = 200000;
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
        adapter = new AdapterForTest(address(controller), address(arpa), address(oracle));

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
            gasExceptRequestDetail,
            fallbackWeiPerUnitArpa,
            signatureTaskExclusiveWindow,
            rewardPerSignature,
            committerRewardPerSignature,
            IAdapterOwner.FeeConfig(250000, 250000, 250000, 250000, 250000, 0, 0, 0, 0)
        );

        vm.prank(stakingDeployer);
        staking.setController(address(controller));

        uint96 plentyOfArpaBalance = 1e6 * 1e18;
        deal(address(arpa), address(admin), 3 * plentyOfArpaBalance);

        changePrank(admin);
        prepareSubscription(address(getRandomNumberExample), plentyOfArpaBalance);
        prepareSubscription(address(rollDiceExample), plentyOfArpaBalance);
        prepareSubscription(address(getShuffledArrayExample), plentyOfArpaBalance);
        prepareAnAvailableGroup();
    }

    function testAdapterAddress() public {
        emit log_address(address(adapter));
        assertEq(getRandomNumberExample.adapter(), address(adapter));
        assertEq(rollDiceExample.adapter(), address(adapter));
        assertEq(getShuffledArrayExample.adapter(), address(adapter));
    }

    function testGetRandomNumber() public {
        deal(user, 1 * 1e18);
        changePrank(user);

        uint32 times = 10;
        for (uint256 i = 0; i < times; i++) {
            bytes32 requestId = getRandomNumberExample.getRandomNumber();

            Adapter.RequestDetail memory rd = adapter.getPendingRequest(requestId);
            bytes memory rawSeed = abi.encodePacked(rd.seed);
            emit log_named_bytes("rawSeed", rawSeed);

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

        Adapter.RequestDetail memory rd = adapter.getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

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

        Adapter.RequestDetail memory rd = adapter.getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

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
        uint64 subId = prepareSubscription(address(advancedGetShuffledArrayExample), plentyOfArpaBalance);

        deal(user, 1 * 1e18);
        changePrank(user);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 0;
        uint256 rdGasLimit = 350000;
        uint256 rdMaxGasPrice = 1 * 1e9;

        bytes32 requestId = advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, rdGasLimit, rdMaxGasPrice
        );

        Adapter.RequestDetail memory rd = adapter.getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

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
