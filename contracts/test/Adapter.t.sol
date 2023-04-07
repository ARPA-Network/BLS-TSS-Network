// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../src/user/examples/GetRandomNumberExample.sol";
import "./RandcastTestHelper.sol";
import "../src/libraries/BLS.sol";

contract AdapterTest is RandcastTestHelper {
    GetRandomNumberExample getRandomNumberExample;
    uint64 subId;

    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 10;
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;

    uint16 minimumRequestConfirmations = 3;
    uint32 maxGasLimit = 2000000;
    uint32 stalenessSeconds = 30;
    uint32 gasAfterPaymentCalculation = 30000;
    uint32 gasExceptCallback = 200000;
    int256 fallbackWeiPerUnitArpa = 1e12;
    uint256 signatureTaskExclusiveWindow = 10;
    uint256 rewardPerSignature = 50;
    uint256 committerRewardPerSignature = 100;

    uint32 fulfillmentFlatFeeArpaPPMTier1 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier2 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier3 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier4 = 250000;
    uint32 fulfillmentFlatFeeArpaPPMTier5 = 250000;
    uint24 reqsForTier2 = 0;
    uint24 reqsForTier3 = 0;
    uint24 reqsForTier4 = 0;
    uint24 reqsForTier5 = 0;

    uint96 plentyOfArpaBalance = 1e6 * 1e18;

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
        controller = new ControllerForTest(address(arpa), address(oracle));

        vm.prank(admin);
        getRandomNumberExample = new GetRandomNumberExample(
            address(controller)
        );

        vm.prank(admin);
        controller.setControllerConfig(
            address(staking),
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

        vm.prank(stakingDeployer);
        staking.setController(address(controller));

        deal(address(arpa), address(user), plentyOfArpaBalance);

        vm.prank(user);
        arpa.approve(address(controller), 3 * plentyOfArpaBalance);

        changePrank(user);
        subId = prepareSubscription(address(getRandomNumberExample), plentyOfArpaBalance);
    }

    function testControllerAddress() public {
        emit log_address(address(controller));
        assertEq(getRandomNumberExample.controller(), address(controller));
    }

    function testUserContractOwner() public {
        emit log_address(address(getRandomNumberExample));
        assertEq(getRandomNumberExample.owner(), admin);
    }

    function testCannotRequestByEOA() public {
        vm.stopPrank();
        deal(user, 1 * 1e18);
        vm.expectRevert(
            abi.encodeWithSelector(
                Adapter.InvalidRequestByEOA.selector,
                "Please request by extending GeneralRandcastConsumerBase so that we can callback with randomness."
            )
        );

        IAdapter.RandomnessRequestParams memory p;
        vm.broadcast(user);
        controller.requestRandomness(p);
    }

    function testRequestRandomness() public {
        prepareAnAvailableGroup();
        deal(user, 1 * 1e18);

        uint32 times = 10;
        for (uint256 i = 0; i < times; i++) {
            vm.startBroadcast(user);
            bytes32 requestId = getRandomNumberExample.getRandomNumber();
            emit log_bytes32(requestId);
            vm.stopBroadcast();
            (, uint96 inflightCost,,,) = controller.getSubscription(subId);
            emit log_uint(inflightCost);

            uint96 payment = controller.estimatePaymentAmount(
                getRandomNumberExample.callbackGasLimit(),
                gasExceptCallback,
                fulfillmentFlatFeeArpaPPMTier1,
                tx.gasprice * 3
            );

            assertEq(inflightCost, payment * (i + 1));

            Controller.Callback memory callback = controller.getPendingRequest(requestId);
            bytes memory actualSeed = abi.encodePacked(callback.seed, callback.blockNum);

            emit log_named_bytes("actualSeed", actualSeed);

            vm.roll(block.number + 1);
        }
    }

    function testFulfillRandomness() public {
        prepareAnAvailableGroup();
        deal(user, 1 * 1e18);

        uint32 times = 1;

        vm.broadcast(user);
        bytes32 requestId = getRandomNumberExample.getRandomNumber();
        emit log_bytes32(requestId);

        vm.startBroadcast(node1);
        fulfillRequest(requestId, 0);
        vm.stopBroadcast();

        vm.roll(block.number + 1);
        assertEq(getRandomNumberExample.randomnessResults(0), controller.lastOutput());
        assertEq(getRandomNumberExample.lengthOfRandomnessResults(), times);
    }
}
