// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "forge-std/Test.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import "../../src/Controller.sol";
import "../../src/Adapter.sol";
import "../../src/interfaces/IControllerOwner.sol";
import "../../src/interfaces/IAdapterOwner.sol";
import "../../src/interfaces/IAdapter.sol";
import {Staking, ArpaTokenInterface} from "Staking-v0.1/Staking.sol";
import "./MockUpgradedAdapter.sol";

contract ProxyTest is Test {
    Controller controller;
    ERC1967Proxy adapter;
    Adapter adapter_impl;
    Staking staking;
    IERC20 arpa;

    address public admin = address(0xABCD);
    address public stakingDeployer = address(0xBCDE);

    // Staking params
    /// @notice The ARPA Token
    ArpaTokenInterface ARPAAddress;
    /// @notice The initial maximum total stake amount across all stakers
    uint256 initialMaxPoolSize = 50_000_00 * 1e18;
    /// @notice The initial maximum stake amount for a single community staker
    uint256 initialMaxCommunityStakeAmount = 2_500_00 * 1e18;
    /// @notice The minimum stake amount that a community staker can stake
    uint256 minCommunityStakeAmount = 1e12;
    /// @notice The minimum stake amount that an operator can stake
    uint256 operatorStakeAmount = 500_00 * 1e18;
    /// @notice The minimum number of node operators required to initialize the
    /// staking pool.
    uint256 minInitialOperatorCount = 1;
    /// @notice The minimum reward duration after pool config updates and pool
    /// reward extensions
    uint256 minRewardDuration = 1 days;
    /// @notice Used to calculate delegated stake amount
    /// = amount / delegation rate denominator = 100% / 100 = 1%
    uint256 delegationRateDenominator = 20;
    /// @notice The freeze duration for stakers after unstaking
    uint256 unstakeFreezingDuration = 14 days;

    // Controller params
    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 10;
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;
    uint256 last_output = 2222222222222222;

    // Adapter params
    uint16 minimumRequestConfirmations = 3;
    uint32 maxGasLimit = 2000000;
    uint32 gasAfterPaymentCalculation = 30000;
    uint32 gasExceptCallback = 530000;
    uint256 signatureTaskExclusiveWindow = 10;
    uint256 rewardPerSignature = 50;
    uint256 committerRewardPerSignature = 100;

    uint32 fulfillmentFlatFeeEthPPMTier1 = 2500000;
    uint32 fulfillmentFlatFeeEthPPMTier2 = 250000;
    uint32 fulfillmentFlatFeeEthPPMTier3 = 25000;
    uint32 fulfillmentFlatFeeEthPPMTier4 = 2500;
    uint32 fulfillmentFlatFeeEthPPMTier5 = 250;
    uint24 reqsForTier2 = 10;
    uint24 reqsForTier3 = 20;
    uint24 reqsForTier4 = 30;
    uint24 reqsForTier5 = 40;

    uint16 flatFeePromotionGlobalPercentage = 100;
    bool isFlatFeePromotionEnabledPermanently = false;
    uint256 flatFeePromotionStartTimestamp = 0;
    uint256 flatFeePromotionEndTimestamp = 0;

    function setUp() public {
        skip(1000);

        vm.prank(admin);
        arpa = new ERC20("arpa token", "ARPA");

        Staking.PoolConstructorParams memory params = Staking.PoolConstructorParams(
            ArpaTokenInterface(address(arpa)),
            initialMaxPoolSize,
            initialMaxCommunityStakeAmount,
            minCommunityStakeAmount,
            operatorStakeAmount,
            minInitialOperatorCount,
            minRewardDuration,
            delegationRateDenominator,
            unstakeFreezingDuration
        );
        vm.prank(admin);
        staking = new Staking(params);

        vm.prank(admin);
        controller = new Controller();

        vm.prank(admin);
        controller.initialize(address(staking), last_output);

        vm.prank(admin);
        adapter_impl = new Adapter();

        vm.prank(admin);
        adapter =
            new ERC1967Proxy(address(adapter_impl), abi.encodeWithSignature("initialize(address)", address(controller)));

        vm.prank(admin);
        IControllerOwner(address(controller)).setControllerConfig(
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
        IAdapterOwner(address(adapter)).setAdapterConfig(
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

        vm.prank(admin);
        staking.setController(address(controller));
    }

    function testAdapterProxy() public {
        assertEq(IAdapter(address(adapter)).getFeeTier(50), fulfillmentFlatFeeEthPPMTier5);

        // adapter cannot be upgraded by anyone except the owner
        vm.prank(stakingDeployer);
        MockUpgradedAdapter adapter_impl_2_by_not_owner = new MockUpgradedAdapter();

        vm.expectRevert("Ownable: caller is not the owner");
        vm.prank(stakingDeployer);
        UUPSUpgradeable(address(adapter)).upgradeTo(address(adapter_impl_2_by_not_owner));

        // adapter can be upgraded by the owner
        vm.prank(admin);
        MockUpgradedAdapter adapter_impl_2 = new MockUpgradedAdapter();

        // initialize should fail because the contract is already initialized(state should be the same)
        vm.expectRevert("Initializable: contract is already initialized");
        vm.prank(admin);
        UUPSUpgradeable(address(adapter)).upgradeToAndCall(
            address(adapter_impl_2), abi.encodeWithSignature("initialize(address)", address(controller))
        );

        vm.prank(admin);
        UUPSUpgradeable(address(adapter)).upgradeTo(address(adapter_impl_2));

        // the state shoud be the same after upgrade
        assertEq(IAdapter(address(adapter)).getFeeTier(50), fulfillmentFlatFeeEthPPMTier5);

        // the authorization should be the same after upgrade
        uint32 newFulfillmentFlatFeeEthPPMTier5 = 25;
        vm.prank(admin);
        IAdapterOwner(address(adapter)).setFlatFeeConfig(
            IAdapterOwner.FeeConfig(
                fulfillmentFlatFeeEthPPMTier1,
                fulfillmentFlatFeeEthPPMTier2,
                fulfillmentFlatFeeEthPPMTier3,
                fulfillmentFlatFeeEthPPMTier4,
                newFulfillmentFlatFeeEthPPMTier5,
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
        assertEq(IAdapter(address(adapter)).getFeeTier(50), newFulfillmentFlatFeeEthPPMTier5);

        // the new function should be called successfully
        (bool success, bytes memory result) =
            address(adapter).call(abi.encodeWithSelector(MockUpgradedAdapter.version.selector));
        assertEq(success, true);
        assertEq(abi.decode(result, (uint256)), 2);
    }
}
