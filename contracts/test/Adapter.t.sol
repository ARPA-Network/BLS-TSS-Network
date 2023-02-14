// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../src/user/examples/GetRandomNumberExample.sol";
import "./RandcastTestHelper.sol";
import "../src/libraries/BLS.sol";

contract AdapterTest is RandcastTestHelper {
    GetRandomNumberExample getRandomNumberExample;
    uint64 subId;

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

    uint96 plentyOfArpaBalance = 1e6 * 1e18;

    function setUp() public {
        skip(1000);

        changePrank(admin);
        arpa = new ERC20("arpa token", "ARPA");
        oracle = new MockArpaEthOracle();
        controller = new Controller(address(arpa), address(oracle));

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

        changePrank(user);
        deal(address(arpa), address(user), plentyOfArpaBalance);
        arpa.approve(address(controller), 3 * plentyOfArpaBalance);

        getRandomNumberExample = new GetRandomNumberExample(
            address(controller)
        );

        subId = prepareSubscription(
            address(getRandomNumberExample),
            plentyOfArpaBalance
        );
    }

    function testControllerAddress() public {
        emit log_address(address(controller));
        assertEq(getRandomNumberExample.controller(), address(controller));
    }

    function testUserContractOwner() public {
        emit log_address(address(getRandomNumberExample));
        assertEq(getRandomNumberExample.owner(), user);
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

        IAdapter.RequestRandomnessParams memory p;
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
            (, uint96 inflightCost, , , ) = controller.getSubscription(subId);
            emit log_uint(inflightCost);

            uint96 payment = controller.estimatePaymentAmount(
                getRandomNumberExample.callbackGasLimit(),
                gasExceptCallback,
                fulfillmentFlatFeeArpaPPMTier1,
                tx.gasprice * 3
            );

            assertEq(inflightCost, payment * (i + 1));

            Controller.Callback memory callback = controller.getPendingRequest(
                requestId
            );
            bytes memory actualSeed = abi.encodePacked(
                callback.seed,
                callback.blockNum
            );

            emit log_named_bytes("actualSeed", actualSeed);

            vm.roll(block.number + 1);
        }
    }

    function testFulfillRandomness() public {
        prepareAnAvailableGroup();
        deal(user, 1 * 1e18);
        deal(node, 1 * 1e18);

        uint32 times = 1;

        vm.startBroadcast(user);
        bytes32 requestId = getRandomNumberExample.getRandomNumber();
        emit log_bytes32(requestId);
        vm.stopBroadcast();

        vm.startBroadcast(node1);
        fulfillRequest(requestId);
        vm.stopBroadcast();

        vm.roll(block.number + 1);
        emit log_uint(getRandomNumberExample.randomnessResults(0));
        assertEq(getRandomNumberExample.lengthOfRandomnessResults(), times);
    }
}
