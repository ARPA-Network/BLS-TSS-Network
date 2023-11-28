// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GetRandomNumberExample} from "Randcast-User-Contract/user/examples/GetRandomNumberExample.sol";
import {GetShuffledArrayExample} from "Randcast-User-Contract/user/examples/GetShuffledArrayExample.sol";
import {RollDiceExample, GeneralRandcastConsumerBase} from "Randcast-User-Contract/user/examples/RollDiceExample.sol";
import {AdvancedGetShuffledArrayExample} from "Randcast-User-Contract/user/examples/AdvancedGetShuffledArrayExample.sol";
import {GeneralRandcastConsumerBase} from "Randcast-User-Contract/user/GeneralRandcastConsumerBase.sol";
import {
    IAdapter,
    Adapter,
    RandcastTestHelper,
    ERC20,
    ControllerForTest,
    AdapterForTest,
    ERC1967Proxy
} from "../RandcastTestHelper.sol";
import {IAdapterOwner} from "../../src/interfaces/IAdapterOwner.sol";

//solhint-disable-next-line max-states-count
contract ConsumerRequestBalanceTest is RandcastTestHelper {
    GetRandomNumberExample internal _getRandomNumberExample;
    GetShuffledArrayExample internal _getShuffledArrayExample;
    RollDiceExample internal _rollDiceExample;
    AdvancedGetShuffledArrayExample internal _advancedGetShuffledArrayExample;

    uint256 internal _disqualifiedNodePenaltyAmount = 1000;
    uint256 internal _defaultNumberOfCommitters = 3;
    uint256 internal _defaultDkgPhaseDuration = 10;
    uint256 internal _groupMaxCapacity = 10;
    uint256 internal _idealNumberOfGroups = 5;
    uint256 internal _pendingBlockAfterQuit = 100;
    uint256 internal _dkgPostProcessReward = 100;
    uint256 internal _lastOutput = 2222222222222222;

    uint16 internal _minimumRequestConfirmations = 6;
    uint32 internal _maxGasLimit = 2000000;
    uint32 internal _gasAfterPaymentCalculation = 50000;
    uint32 internal _gasExceptCallback = 550000;
    uint256 internal _signatureTaskExclusiveWindow = 10;
    uint256 internal _rewardPerSignature = 50;
    uint256 internal _committerRewardPerSignature = 100;

    uint16 internal _flatFeePromotionGlobalPercentage = 100;
    bool internal _isFlatFeePromotionEnabledPermanently = false;
    uint256 internal _flatFeePromotionStartTimestamp = 0;
    uint256 internal _flatFeePromotionEndTimestamp = 0;

    function setUp() public {
        skip(1000);
        vm.prank(_admin);
        _arpa = new ERC20("arpa token", "ARPA");

        address[] memory operators = new address[](5);
        operators[0] = _node1;
        operators[1] = _node2;
        operators[2] = _node3;
        operators[3] = _node4;
        operators[4] = _node5;
        _prepareStakingContract(_stakingDeployer, address(_arpa), operators);

        vm.prank(_admin);
        _controller = new ControllerForTest(address(_arpa), _lastOutput);

        vm.prank(_admin);
        _adapterImpl = new AdapterForTest();

        vm.prank(_admin);
        _adapter =
            new ERC1967Proxy(address(_adapterImpl),abi.encodeWithSignature("initialize(address)",address(_controller)));

        vm.prank(_user);
        _getRandomNumberExample = new GetRandomNumberExample(
            address(_adapter)
        );

        vm.prank(_user);
        _rollDiceExample = new RollDiceExample(address(_adapter));

        vm.prank(_user);
        _getShuffledArrayExample = new GetShuffledArrayExample(
            address(_adapter)
        );

        vm.prank(_user);
        _advancedGetShuffledArrayExample = new AdvancedGetShuffledArrayExample(
            address(_adapter)
        );

        vm.prank(_admin);
        _controller.setControllerConfig(
            address(_staking),
            address(_adapter),
            _operatorStakeAmount,
            _disqualifiedNodePenaltyAmount,
            _defaultNumberOfCommitters,
            _defaultDkgPhaseDuration,
            _groupMaxCapacity,
            _idealNumberOfGroups,
            _pendingBlockAfterQuit,
            _dkgPostProcessReward
        );

        vm.prank(_admin);
        IAdapterOwner(address(_adapter)).setAdapterConfig(
            _minimumRequestConfirmations,
            _maxGasLimit,
            _gasAfterPaymentCalculation,
            _gasExceptCallback,
            _signatureTaskExclusiveWindow,
            _rewardPerSignature,
            _committerRewardPerSignature
        );

        vm.prank(_admin);
        IAdapterOwner(address(_adapter)).setFlatFeeConfig(
            IAdapterOwner.FeeConfig(250000, 250000, 250000, 250000, 250000, 0, 0, 0, 0),
            _flatFeePromotionGlobalPercentage,
            _isFlatFeePromotionEnabledPermanently,
            _flatFeePromotionStartTimestamp,
            _flatFeePromotionEndTimestamp
        );

        vm.prank(_stakingDeployer);
        _staking.setController(address(_controller));

        prepareAnAvailableGroup();
    }

    function testAdapterAddress() public {
        emit log_address(address(_adapter));
        assertEq(_rollDiceExample.adapter(), address(_adapter));
    }

    function testCannotRequestWithoutSubscription() public {
        deal(_user, 1 * 1e18);
        vm.startPrank(_user);

        uint32 bunch = 10;
        vm.expectRevert(GeneralRandcastConsumerBase.NoSubscriptionBound.selector);
        _rollDiceExample.rollDice(bunch);
    }

    function testCannotRequestWithoutEnoughBalance() public {
        deal(_user, 1 * 1e18);

        uint256 feweTHBalance = 1 * 1e10;

        _prepareSubscription(_user, address(_rollDiceExample), feweTHBalance);

        uint32 bunch = 10;
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(_user);
        _rollDiceExample.rollDice(bunch);
    }

    function testRequestGeneralExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(_user, 10 * 1e18);
        // (1e18 arpa wei/arpa) (wei/gas * gas) / (wei/arpa) = arpa wei
        // paymentNoFee = (1e18 *
        //     weiPerUnitGas *
        //     (gasExceptCallback + callbackGasUsed) /
        //     uint256(weiPerUnitArpa);
        // callbackGasUsed = 501728 gas
        // WeiPerUnitArpa = 1e12 wei/arpa
        // weiPerUnitGas = 1e9 wei/gas
        // flat fee = 250000 1e12 arpa wei
        // Actual: 904212000000000000000
        // Expected: 891728000000000000000
        uint256 expectedPayment = 1e9 * (uint256(_gasExceptCallback) + 501728);

        uint256 plentyOfEthBalance = 1e16;
        // prepare subId 2 for _rollDiceExample
        IAdapter(address(_adapter)).createSubscription();
        uint64 subId = _prepareSubscription(_user, address(_rollDiceExample), plentyOfEthBalance);

        uint32 bunch = 10;
        vm.prank(_user);
        bytes32 requestId = _rollDiceExample.rollDice(bunch);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 10);

        (uint256 afterBalance, uint256 inflightCost) = _getBalance(subId);

        // the upper limit of delta is 5%
        // maxPercentDelta is an 18 decimal fixed point number, where 1e18 == 100%
        assertApproxEqRel(plentyOfEthBalance - afterBalance, expectedPayment, 1e18 / 20);
        // inflight cost should be 0 after fulfillment
        assertEq(inflightCost, 0);

        for (uint256 i = 0; i < _rollDiceExample.lengthOfDiceResults(); i++) {
            assertTrue(_rollDiceExample.diceResults(i) > 0 && _rollDiceExample.diceResults(i) <= 6);
        }
        assertEq(_rollDiceExample.lengthOfDiceResults(), bunch);
    }

    function testCannotRequestWithTooMuchInflightCost() public {
        deal(_user, 1 * 1e18);

        // give the balance just enough for one request
        // give more than 3 times actual payment since we estimate 3 times max gas fee
        // (501728+30000) + 50000 * (5-3) + 550000 + 9000*5 = 1226728
        uint256 someEthBalance = 1230 * 3 * 1e12;
        _prepareSubscription(_user, address(_rollDiceExample), someEthBalance);
        uint32 bunch = 10;
        vm.prank(_user);
        _rollDiceExample.rollDice(bunch);
        // now we have an inflight request, then try to request again in the next block
        vm.roll(block.number + 1);
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(_user);
        _rollDiceExample.rollDice(bunch);
    }

    function testRequestAdvancedExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(_user, 1 * 1e18);
        // 350000 + 50000 * (5-3) + 550000 + 9000*5 = 1045000
        uint256 plentyOfEthBalance = 1050e12;
        // prepare subId 4 for _advancedGetShuffledArrayExample
        IAdapter(address(_adapter)).createSubscription();
        IAdapter(address(_adapter)).createSubscription();
        IAdapter(address(_adapter)).createSubscription();
        uint64 subId = _prepareSubscription(_user, address(_advancedGetShuffledArrayExample), plentyOfEthBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 6;
        // just cover actual gasused
        uint32 callbackGasLimit = 350000;
        uint256 callbackMaxGasPrice = 1 * 1e9;

        vm.prank(_user);
        bytes32 requestId = _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 12);

        assertEq(_advancedGetShuffledArrayExample.lengthOfShuffleResults(), 1);

        for (uint256 k = 0; k < _advancedGetShuffledArrayExample.lengthOfShuffleResults(); k++) {
            for (uint256 i = 0; i < upper; i++) {
                assertTrue(
                    _advancedGetShuffledArrayExample.shuffleResults(k, i) >= 0
                        && _advancedGetShuffledArrayExample.shuffleResults(k, i) < upper
                );
            }
        }
    }

    function testCannotRequestAdvancedExampleWithTooHighCallbackGasLimitAndCallbackMaxGasFee() public {
        deal(_user, 1 * 1e18);

        uint256 someEthBalance = 1e18;
        uint64 subId = _prepareSubscription(_user, address(_advancedGetShuffledArrayExample), someEthBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 6;
        uint32 callbackGasLimit = 2e6;
        uint256 callbackMaxGasPrice = 1e3 * 1e9;
        // payment = 2e6 * 1e3 * 1e9 = 2e18

        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(_user);
        _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );
    }
}
