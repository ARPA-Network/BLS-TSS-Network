// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {SharedConsumer} from "Randcast-User-Contract/user/SharedConsumer.sol";
import {ISharedConsumer} from "Randcast-User-Contract/interfaces/ISharedConsumer.sol";
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
contract RandcastPlaygroundConsumerTest is RandcastTestHelper {
    ERC1967Proxy internal _shareConsumer;

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
        SharedConsumer _shareConsumerImpl = new SharedConsumer(address(_adapter));

        vm.prank(_user);
        _shareConsumer = new ERC1967Proxy(address(_shareConsumerImpl),abi.encodeWithSignature("initialize()"));

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
        _prepareSubscription(_admin, address(_shareConsumer), plentyOfEthBalance);

        prepareAnAvailableGroup();
    }

    function testPlaygroundDrawTickets() public {
        deal(_user, 1 * 1e18);
        vm.prank(_user);
        ISharedConsumer(address(_shareConsumer)).setTrialSubscription(5);

        uint32 ticketNumber = 30;
        uint32 winnerNumber = 10;

        vm.prank(_user);
        uint256 gasFee = ISharedConsumer(address(_shareConsumer)).estimateFee(
            ISharedConsumer.PlayType.Draw, 0, abi.encode(ticketNumber, winnerNumber)
        );

        vm.prank(_user);
        bytes32 requestId =
            ISharedConsumer(address(_shareConsumer)).drawTickets{value: gasFee}(ticketNumber, winnerNumber, 0, 0, 6);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 17);
        (,, uint256 balance,,,,,,) = AdapterForTest(address(_adapter)).getSubscription(2);

        vm.prank(_user);
        uint256 curBalance = _user.balance;
        ISharedConsumer(address(_shareConsumer)).cancelSubscription();
        assertEq(_user.balance, curBalance + balance);
        emit log_uint(gasFee);
    }

    function testPlaygroundRollDice() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        ISharedConsumer(address(_shareConsumer)).setTrialSubscription(5);

        uint32 bunch = 1;
        uint32 size = 6;

        vm.prank(_user);
        uint256 gasFee = ISharedConsumer(address(_shareConsumer)).estimateFee(
            ISharedConsumer.PlayType.Roll, 0, abi.encode(bunch, size)
        );

        vm.prank(_user);
        bytes32 requestId = ISharedConsumer(address(_shareConsumer)).rollDice{value: gasFee}(bunch, size, 0, 0, 0);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 17);
        (,, uint256 balance,,,,,,) = AdapterForTest(address(_adapter)).getSubscription(2);

        vm.prank(_user);
        uint256 curBalance = _user.balance;

        ISharedConsumer(address(_shareConsumer)).cancelSubscription();
        assertEq(_user.balance, curBalance + balance);
        emit log_uint(gasFee);
    }

    function testUseSharedSubscription() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        ISharedConsumer(address(_shareConsumer)).setTrialSubscription(1);

        uint32 ticketNumber = 30;
        uint32 winnerNumber = 1;

        vm.prank(_user);
        bytes32 requestId = ISharedConsumer(address(_shareConsumer)).drawTickets(ticketNumber, winnerNumber, 1, 0, 0);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 18);
    }
}
