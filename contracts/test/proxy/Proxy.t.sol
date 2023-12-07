// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Test} from "forge-std/Test.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {UUPSUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import {Controller} from "../../src/Controller.sol";
import {Adapter} from "../../src/Adapter.sol";
import {IControllerOwner} from "../../src/interfaces/IControllerOwner.sol";
import {IAdapterOwner} from "../../src/interfaces/IAdapterOwner.sol";
import {IAdapter} from "../../src/interfaces/IAdapter.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import {MockUpgradedAdapter} from "./MockUpgradedAdapter.sol";

// solhint-disable-next-line max-states-count
contract ProxyTest is Test {
    Controller internal _controller;
    ERC1967Proxy internal _adapter;
    Adapter internal _adapterImpl;
    Staking internal _staking;
    IERC20 internal _arpa;

    address internal _admin = address(0xABCD);
    address internal _stakingDeployer = address(0xBCDE);

    // Staking params
    uint256 internal _initialMaxPoolSize = 50_000_00 * 1e18;
    uint256 internal _initialMaxCommunityStakeAmount = 2_500_00 * 1e18;
    uint256 internal _minCommunityStakeAmount = 1e12;
    uint256 internal _operatorStakeAmount = 500_00 * 1e18;
    uint256 internal _minInitialOperatorCount = 1;
    uint256 internal _minRewardDuration = 1 days;
    uint256 internal _delegationRateDenominator = 20;
    uint256 internal _unstakeFreezingDuration = 14 days;

    // Controller params
    uint256 internal _disqualifiedNodePenaltyAmount = 1000;
    uint256 internal _defaultNumberOfCommitters = 3;
    uint256 internal _defaultDkgPhaseDuration = 10;
    uint256 internal _groupMaxCapacity = 10;
    uint256 internal _idealNumberOfGroups = 5;
    uint256 internal _pendingBlockAfterQuit = 100;
    uint256 internal _dkgPostProcessReward = 100;
    uint256 internal _lastOutput = 2222222222222222;

    // Adapter params
    uint16 internal _minimumRequestConfirmations = 3;
    uint32 internal _maxGasLimit = 2000000;
    uint32 internal _gasAfterPaymentCalculation = 50000;
    uint32 internal _gasExceptCallback = 550000;
    uint256 internal _signatureTaskExclusiveWindow = 10;
    uint256 internal _rewardPerSignature = 50;
    uint256 internal _committerRewardPerSignature = 100;

    uint32 internal _fulfillmentFlatFeeEthPPMTier1 = 2500000;
    uint32 internal _fulfillmentFlatFeeEthPPMTier2 = 250000;
    uint32 internal _fulfillmentFlatFeeEthPPMTier3 = 25000;
    uint32 internal _fulfillmentFlatFeeEthPPMTier4 = 2500;
    uint32 internal _fulfillmentFlatFeeEthPPMTier5 = 250;
    uint24 internal _reqsForTier2 = 10;
    uint24 internal _reqsForTier3 = 20;
    uint24 internal _reqsForTier4 = 30;
    uint24 internal _reqsForTier5 = 40;

    uint16 internal _flatFeePromotionGlobalPercentage = 100;
    bool internal _isFlatFeePromotionEnabledPermanently = false;
    uint256 internal _flatFeePromotionStartTimestamp = 0;
    uint256 internal _flatFeePromotionEndTimestamp = 0;

    function setUp() public {
        skip(1000);

        vm.prank(_admin);
        _arpa = new ERC20("_arpa token", "ARPA");

        Staking.PoolConstructorParams memory params = Staking.PoolConstructorParams(
            IERC20(address(_arpa)),
            _initialMaxPoolSize,
            _initialMaxCommunityStakeAmount,
            _minCommunityStakeAmount,
            _operatorStakeAmount,
            _minInitialOperatorCount,
            _minRewardDuration,
            _delegationRateDenominator,
            _unstakeFreezingDuration
        );
        vm.prank(_admin);
        _staking = new Staking(params);

        vm.prank(_admin);
        _controller = new Controller();

        vm.prank(_admin);
        _controller.initialize(address(_staking), _lastOutput);

        vm.prank(_admin);
        _adapterImpl = new Adapter();

        vm.prank(_admin);
        _adapter =
        new ERC1967Proxy(address(_adapterImpl), abi.encodeWithSignature("initialize(address)", address(_controller)));

        vm.prank(_admin);
        IControllerOwner(address(_controller)).setControllerConfig(
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

        vm.prank(_admin);
        _staking.setController(address(_controller));
    }

    function testAdapterProxy() public {
        assertEq(IAdapter(address(_adapter)).getFeeTier(50), _fulfillmentFlatFeeEthPPMTier5);

        // _adapter cannot be upgraded by anyone except the owner
        vm.prank(_stakingDeployer);
        MockUpgradedAdapter adapterImpl2ByNotOwner = new MockUpgradedAdapter();

        vm.expectRevert("Ownable: caller is not the owner");
        vm.prank(_stakingDeployer);
        UUPSUpgradeable(address(_adapter)).upgradeTo(address(adapterImpl2ByNotOwner));

        // _adapter can be upgraded by the owner
        vm.prank(_admin);
        MockUpgradedAdapter adapterImpl2 = new MockUpgradedAdapter();

        // initialize should fail because the contract is already initialized(state should be the same)
        vm.expectRevert("Initializable: contract is already initialized");
        vm.prank(_admin);
        UUPSUpgradeable(address(_adapter)).upgradeToAndCall(
            address(adapterImpl2), abi.encodeWithSignature("initialize(address)", address(_controller))
        );

        vm.prank(_admin);
        UUPSUpgradeable(address(_adapter)).upgradeTo(address(adapterImpl2));

        // the state shoud be the same after upgrade
        assertEq(IAdapter(address(_adapter)).getFeeTier(50), _fulfillmentFlatFeeEthPPMTier5);

        // the authorization should be the same after upgrade
        uint32 newFulfillmentFlatFeeEthPPMTier5 = 25;
        vm.prank(_admin);
        IAdapterOwner(address(_adapter)).setFlatFeeConfig(
            IAdapterOwner.FeeConfig(
                _fulfillmentFlatFeeEthPPMTier1,
                _fulfillmentFlatFeeEthPPMTier2,
                _fulfillmentFlatFeeEthPPMTier3,
                _fulfillmentFlatFeeEthPPMTier4,
                newFulfillmentFlatFeeEthPPMTier5,
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
        assertEq(IAdapter(address(_adapter)).getFeeTier(50), newFulfillmentFlatFeeEthPPMTier5);

        // the new function should be called successfully
        (bool success, bytes memory result) =
        // solhint-disable-next-line avoid-low-level-calls
         address(_adapter).call(abi.encodeWithSelector(MockUpgradedAdapter.version.selector));
        assertEq(success, true);
        assertEq(abi.decode(result, (uint256)), 2);
    }
}
