// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import {GetRandomNumberExample} from "../src/user/examples/GetRandomNumberExample.sol";
import {IAdapterOwner} from "../src/interfaces/IAdapterOwner.sol";
import {RandcastTestHelper, IAdapter, Adapter, ControllerForTest, AdapterForTest} from "./RandcastTestHelper.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

// solhint-disable-next-line max-states-count
contract AdapterTest is RandcastTestHelper {
    GetRandomNumberExample internal _getRandomNumberExample;
    uint64 internal _subId;

    uint256 internal _disqualifiedNodePenaltyAmount = 1000;
    uint256 internal _defaultNumberOfCommitters = 3;
    uint256 internal _defaultDkgPhaseDuration = 10;
    uint256 internal _groupMaxCapacity = 10;
    uint256 internal _idealNumberOfGroups = 5;
    uint256 internal _pendingBlockAfterQuit = 100;
    uint256 internal _dkgPostProcessReward = 100;
    uint256 internal _lastOutput = 2222222222222222;

    uint16 internal _minimumRequestConfirmations = 3;
    uint32 internal _maxGasLimit = 2000000;
    uint32 internal _gasAfterPaymentCalculation = 30000;
    uint32 internal _gasExceptCallback = 530000;
    uint256 internal _signatureTaskExclusiveWindow = 10;
    uint256 internal _rewardPerSignature = 50;
    uint256 internal _committerRewardPerSignature = 100;

    uint32 internal _fulfillmentFlatFeeEthPPMTier1 = 250000;
    uint32 internal _fulfillmentFlatFeeEthPPMTier2 = 250000;
    uint32 internal _fulfillmentFlatFeeEthPPMTier3 = 250000;
    uint32 internal _fulfillmentFlatFeeEthPPMTier4 = 250000;
    uint32 internal _fulfillmentFlatFeeEthPPMTier5 = 250000;
    uint24 internal _reqsForTier2 = 0;
    uint24 internal _reqsForTier3 = 0;
    uint24 internal _reqsForTier4 = 0;
    uint24 internal _reqsForTier5 = 0;

    uint16 internal _flatFeePromotionGlobalPercentage = 100;
    bool internal _isFlatFeePromotionEnabledPermanently = false;
    uint256 internal _flatFeePromotionStartTimestamp = 0;
    uint256 internal _flatFeePromotionEndTimestamp = 0;

    uint256 internal _plentyOfEthBalance = 1e6 * 1e18;

    function setUp() public {
        skip(1000);

        vm.prank(_admin);
        _arpa = new ERC20("_arpa token", "ARPA");

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
        _adapter = new AdapterForTest(address(_controller));

        vm.prank(_user);
        _getRandomNumberExample = new GetRandomNumberExample(
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
        _adapter.setAdapterConfig(
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
            IAdapterOwner.FeeConfig(
                _fulfillmentFlatFeeEthPPMTier1,
                _fulfillmentFlatFeeEthPPMTier2,
                _fulfillmentFlatFeeEthPPMTier3,
                _fulfillmentFlatFeeEthPPMTier4,
                _fulfillmentFlatFeeEthPPMTier5,
                _reqsForTier2,
                _reqsForTier3,
                _reqsForTier4,
                _reqsForTier5
            ),
            _flatFeePromotionGlobalPercentage,
            _isFlatFeePromotionEnabledPermanently,
            _flatFeePromotionStartTimestamp,
            _flatFeePromotionEndTimestamp
        );

        vm.prank(_stakingDeployer);
        _staking.setController(address(_controller));

        _subId = _prepareSubscription(_user, address(_getRandomNumberExample), _plentyOfEthBalance);
    }

    function testAdapterAddress() public {
        emit log_address(address(_adapter));
        assertEq(_getRandomNumberExample.adapter(), address(_adapter));
    }

    function testUserContractOwner() public {
        emit log_address(address(_getRandomNumberExample));
        assertEq(_getRandomNumberExample.owner(), _user);
    }

    function testCannotRequestByEOA() public {
        deal(_user, 1 * 1e18);
        vm.expectRevert(abi.encodeWithSelector(Adapter.InvalidRequestByEOA.selector));

        IAdapter.RandomnessRequestParams memory p;
        vm.broadcast(_user);
        _adapter.requestRandomness(p);
    }

    function testRequestRandomness() public {
        uint256 threshold = prepareAnAvailableGroup();
        deal(_user, 1 * 1e18);

        uint32 times = 10;
        uint256 _inflightCost;

        for (uint256 i = 0; i < times; i++) {
            vm.prank(_user);
            bytes32 requestId = _getRandomNumberExample.getRandomNumber();
            emit log_bytes32(requestId);
            (, uint256 inflightCost,,,) = _adapter.getSubscription(_subId);
            emit log_uint(inflightCost);

            // 0 flat fee until the first request is actually fulfilled
            uint256 payment = _adapter.estimatePaymentAmountInETH(
                _getRandomNumberExample.callbackGasLimit() + _adapter.RANDOMNESS_REWARD_GAS() * threshold,
                _gasExceptCallback,
                0,
                tx.gasprice * 3
            );

            _inflightCost += payment;

            assertEq(inflightCost, _inflightCost);

            Adapter.RequestDetail memory rd = _adapter.getPendingRequest(requestId);
            bytes memory actualSeed = abi.encodePacked(rd.seed, rd.blockNum);

            emit log_named_bytes("actualSeed", actualSeed);

            vm.roll(block.number + 1);
        }
    }

    function testFulfillRandomness() public {
        prepareAnAvailableGroup();
        deal(_user, 1 * 1e18);

        uint32 times = 1;

        vm.broadcast(_user);
        bytes32 requestId = _getRandomNumberExample.getRandomNumber();
        emit log_bytes32(requestId);

        Adapter.RequestDetail memory rd = _adapter.getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        _fulfillRequest(_node1, requestId, 0);

        vm.roll(block.number + 1);
        assertEq(_getRandomNumberExample.randomnessResults(0), _adapter.getLastRandomness());
        assertEq(_getRandomNumberExample.lengthOfRandomnessResults(), times);
    }
}
