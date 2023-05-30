// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import {GetRandomNumberExample} from "../src/user/examples/GetRandomNumberExample.sol";
import {IAdapterOwner} from "../src/interfaces/IAdapterOwner.sol";
import {RandcastTestHelper, IAdapter, Adapter, ControllerForTest, AdapterForTest} from "./RandcastTestHelper.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

// solhint-disable-next-line max-states-count
contract AdapterTest is RandcastTestHelper {
    GetRandomNumberExample internal getRandomNumberExample;
    uint64 internal subId;

    uint256 internal disqualifiedNodePenaltyAmount = 1000;
    uint256 internal defaultNumberOfCommitters = 3;
    uint256 internal defaultDkgPhaseDuration = 10;
    uint256 internal groupMaxCapacity = 10;
    uint256 internal idealNumberOfGroups = 5;
    uint256 internal pendingBlockAfterQuit = 100;
    uint256 internal dkgPostProcessReward = 100;
    uint256 internal lastOutput = 2222222222222222;

    uint16 internal minimumRequestConfirmations = 3;
    uint32 internal maxGasLimit = 2000000;
    uint32 internal gasAfterPaymentCalculation = 30000;
    uint32 internal gasExceptCallback = 530000;
    uint256 internal signatureTaskExclusiveWindow = 10;
    uint256 internal rewardPerSignature = 50;
    uint256 internal committerRewardPerSignature = 100;

    uint32 internal fulfillmentFlatFeeEthPPMTier1 = 250000;
    uint32 internal fulfillmentFlatFeeEthPPMTier2 = 250000;
    uint32 internal fulfillmentFlatFeeEthPPMTier3 = 250000;
    uint32 internal fulfillmentFlatFeeEthPPMTier4 = 250000;
    uint32 internal fulfillmentFlatFeeEthPPMTier5 = 250000;
    uint24 internal reqsForTier2 = 0;
    uint24 internal reqsForTier3 = 0;
    uint24 internal reqsForTier4 = 0;
    uint24 internal reqsForTier5 = 0;

    uint16 internal flatFeePromotionGlobalPercentage = 100;
    bool internal isFlatFeePromotionEnabledPermanently = false;
    uint256 internal flatFeePromotionStartTimestamp = 0;
    uint256 internal flatFeePromotionEndTimestamp = 0;

    uint256 internal plentyOfEthBalance = 1e6 * 1e18;

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
        controller = new ControllerForTest(address(arpa), lastOutput);

        vm.prank(admin);
        adapter = new AdapterForTest(address(controller));

        vm.prank(user);
        getRandomNumberExample = new GetRandomNumberExample(
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
            IAdapterOwner.FeeConfig(
                fulfillmentFlatFeeEthPPMTier1,
                fulfillmentFlatFeeEthPPMTier2,
                fulfillmentFlatFeeEthPPMTier3,
                fulfillmentFlatFeeEthPPMTier4,
                fulfillmentFlatFeeEthPPMTier5,
                reqsForTier2,
                reqsForTier3,
                reqsForTier4,
                reqsForTier5
            ),
            flatFeePromotionGlobalPercentage,
            isFlatFeePromotionEnabledPermanently,
            flatFeePromotionStartTimestamp,
            flatFeePromotionEndTimestamp
        );

        vm.prank(stakingDeployer);
        staking.setController(address(controller));

        subId = _prepareSubscription(user, address(getRandomNumberExample), plentyOfEthBalance);
    }

    function testAdapterAddress() public {
        emit log_address(address(adapter));
        assertEq(getRandomNumberExample.adapter(), address(adapter));
    }

    function testUserContractOwner() public {
        emit log_address(address(getRandomNumberExample));
        assertEq(getRandomNumberExample.owner(), user);
    }

    function testCannotRequestByEOA() public {
        deal(user, 1 * 1e18);
        vm.expectRevert(abi.encodeWithSelector(Adapter.InvalidRequestByEOA.selector));

        IAdapter.RandomnessRequestParams memory p;
        vm.broadcast(user);
        adapter.requestRandomness(p);
    }

    function testRequestRandomness() public {
        uint256 threshold = prepareAnAvailableGroup();
        deal(user, 1 * 1e18);

        uint32 times = 10;
        uint256 _inflightCost;

        for (uint256 i = 0; i < times; i++) {
            vm.prank(user);
            bytes32 requestId = getRandomNumberExample.getRandomNumber();
            emit log_bytes32(requestId);
            (, uint256 inflightCost,,,) = adapter.getSubscription(subId);
            emit log_uint(inflightCost);

            // 0 flat fee until the first request is actually fulfilled
            uint256 payment = adapter.estimatePaymentAmountInETH(
                getRandomNumberExample.callbackGasLimit() + adapter.RANDOMNESS_REWARD_GAS() * threshold,
                gasExceptCallback,
                0,
                tx.gasprice * 3
            );

            _inflightCost += payment;

            assertEq(inflightCost, _inflightCost);

            Adapter.RequestDetail memory rd = adapter.getPendingRequest(requestId);
            bytes memory actualSeed = abi.encodePacked(rd.seed, rd.blockNum);

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

        Adapter.RequestDetail memory rd = adapter.getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        fulfillRequest(node1, requestId, 0);

        vm.roll(block.number + 1);
        assertEq(getRandomNumberExample.randomnessResults(0), adapter.getLastRandomness());
        assertEq(getRandomNumberExample.lengthOfRandomnessResults(), times);
    }
}
