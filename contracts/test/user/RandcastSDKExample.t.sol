// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {DrawLotteryExample} from "Randcast-User-Contract/user/examples/DrawLotteryExample.sol";
import {PickRarityExample} from "Randcast-User-Contract/user/examples/PickRarityExample.sol";
import {PickPropertyExample} from "Randcast-User-Contract/user/examples/PickPropertyExample.sol";
import {PickWinnerExample} from "Randcast-User-Contract/user/examples/PickWinnerExample.sol";

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
contract RandcastSDKExampleTest is RandcastTestHelper {
    DrawLotteryExample internal _drawLotteryExample;
    PickRarityExample internal _pickRarityExample;
    PickPropertyExample internal _pickPropertyExample;
    PickWinnerExample internal _pickWinnerExample;

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
        _drawLotteryExample = new DrawLotteryExample(
            address(_adapter)
        );

        vm.prank(_user);
        _pickRarityExample = new PickRarityExample(
            address(_adapter)
        );

        vm.prank(_user);
        _pickPropertyExample = new PickPropertyExample(
            address(_adapter)
        );

        vm.prank(_user);
        _pickWinnerExample = new PickWinnerExample(
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

        _prepareSubscription(_admin, address(_drawLotteryExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_pickRarityExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_pickPropertyExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_pickWinnerExample), plentyOfEthBalance);

        prepareAnAvailableGroup();
    }

    function testDrawLottery() public {
        deal(_user, 1 * 1e18);

        uint32 ticketNumber = 10;
        uint32 winnerNumber = 2;

        vm.prank(_user);
        bytes32 requestId = _drawLotteryExample.getTickets(ticketNumber, winnerNumber);
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 13);

        uint256[] memory ticketResults = _drawLotteryExample.getTicketResults(requestId);
        for (uint256 i = 0; i < winnerNumber; i++) {
            emit log_uint(_drawLotteryExample.winnerResults(i));
            bool winnerInTickets = false;
            for (uint256 j = 0; j < ticketResults.length; j++) {
                if (_drawLotteryExample.winnerResults(i) == ticketResults[j]) {
                    winnerInTickets = true;
                    break;
                }
            }
            assertTrue(winnerInTickets);
        }
        assertEq(_drawLotteryExample.lengthOfWinnerResults(), winnerNumber);
    }

    function testPickRarity() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        bytes32 requestId = _pickRarityExample.getRarity();
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 14);
        assertTrue(_pickRarityExample.indexResult() < 5);
    }

    function testPickProperty() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        bytes32 requestId = _pickPropertyExample.getProperty();
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 15);
        assertTrue(_pickPropertyExample.indexResult() < 3);
    }

    function testPickWinner() public {
        deal(_user, 1 * 1e18);
        vm.prank(_user);
        bytes32 requestId = _pickWinnerExample.getWinner();
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 16);
        assertTrue(_pickWinnerExample.indexResult() < 3);
    }
}
