// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GetRandomNumberExample} from "Randcast-User-Contract/user/examples/GetRandomNumberExample.sol";
import {GetShuffledArrayExample} from "Randcast-User-Contract/user/examples/GetShuffledArrayExample.sol";
import {RollDiceExample} from "Randcast-User-Contract/user/examples/RollDiceExample.sol";
import {AdvancedGetShuffledArrayExample} from "Randcast-User-Contract/user/examples/AdvancedGetShuffledArrayExample.sol";
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
contract RandcastConsumerExampleTest is RandcastTestHelper {
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

        vm.broadcast(_admin);
        IAdapterOwner(address(_adapter)).setFlatFeeConfig(
            IAdapterOwner.FeeConfig(250000, 250000, 250000, 250000, 250000, 0, 0, 0, 0),
            _flatFeePromotionGlobalPercentage,
            _isFlatFeePromotionEnabledPermanently,
            _flatFeePromotionStartTimestamp,
            _flatFeePromotionEndTimestamp
        );

        vm.prank(_stakingDeployer);
        _staking.setController(address(_controller));

        uint256 plentyOfEthBalance = 1e6 * 1e18;

        _prepareSubscription(_admin, address(_getRandomNumberExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_rollDiceExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_getShuffledArrayExample), plentyOfEthBalance);
        prepareAnAvailableGroup();
    }

    function testAdapterAddress() public {
        emit log_address(address(_adapter));
        assertEq(_getRandomNumberExample.adapter(), address(_adapter));
        assertEq(_rollDiceExample.adapter(), address(_adapter));
        assertEq(_getShuffledArrayExample.adapter(), address(_adapter));
    }

    function testGetRandomNumber() public {
        deal(_user, 1 * 1e18);

        uint32 times = 10;
        for (uint256 i = 0; i < times; i++) {
            vm.prank(_user);
            bytes32 requestId = _getRandomNumberExample.getRandomNumber();

            Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
            bytes memory rawSeed = abi.encodePacked(rd.seed);
            emit log_named_bytes("rawSeed", rawSeed);

            deal(_node1, 1 * 1e18);
            _fulfillRequest(_node1, requestId, i);

            vm.roll(block.number + 1);
        }

        for (uint256 i = 0; i < _getRandomNumberExample.lengthOfRandomnessResults(); i++) {
            emit log_uint(_getRandomNumberExample.randomnessResults(i));
        }
        assertEq(_getRandomNumberExample.lengthOfRandomnessResults(), times);
    }

    function testRollDice() public {
        deal(_user, 1 * 1e18);

        uint32 bunch = 10;
        vm.prank(_user);
        bytes32 requestId = _rollDiceExample.rollDice(bunch);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 10);

        for (uint256 i = 0; i < _rollDiceExample.lengthOfDiceResults(); i++) {
            emit log_uint(_rollDiceExample.diceResults(i));
            assertTrue(_rollDiceExample.diceResults(i) > 0 && _rollDiceExample.diceResults(i) <= 6);
        }
        assertEq(_rollDiceExample.lengthOfDiceResults(), bunch);
    }

    function testGetShuffledArray() public {
        deal(_user, 1 * 1e18);

        uint32 upper = 10;
        vm.prank(_user);
        bytes32 requestId = _getShuffledArrayExample.getShuffledArray(upper);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 11);

        for (uint256 i = 0; i < upper; i++) {
            emit log_uint(_getShuffledArrayExample.shuffleResults(i));
            assertTrue(
                _getShuffledArrayExample.shuffleResults(i) >= 0 && _getShuffledArrayExample.shuffleResults(i) < upper
            );
        }
        assertEq(_getShuffledArrayExample.lengthOfShuffleResults(), upper);
    }

    function testAdvancedGetShuffledArray() public {
        uint256 plentyOfEthBalance = 1e6 * 1e18;
        uint64 subId = _prepareSubscription(_admin, address(_advancedGetShuffledArrayExample), plentyOfEthBalance);

        deal(_user, 1 * 1e18);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 6;
        uint32 rdGasLimit = 350000;
        uint256 rdMaxGasPrice = 1 * 1e9;

        vm.prank(_user);
        bytes32 requestId = _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, rdGasLimit, rdMaxGasPrice
        );

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 12);

        assertEq(_advancedGetShuffledArrayExample.lengthOfShuffleResults(), 1);

        for (uint256 k = 0; k < _advancedGetShuffledArrayExample.lengthOfShuffleResults(); k++) {
            for (uint256 i = 0; i < upper; i++) {
                emit log_uint(_advancedGetShuffledArrayExample.shuffleResults(k, i));
                assertTrue(
                    _advancedGetShuffledArrayExample.shuffleResults(k, i) >= 0
                        && _advancedGetShuffledArrayExample.shuffleResults(k, i) < upper
                );
            }
        }
    }
}
