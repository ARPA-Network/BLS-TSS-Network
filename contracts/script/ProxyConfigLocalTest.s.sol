// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/interfaces/IAdapter.sol";
import "../src/Controller.sol";
import "../src/Proxy.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

contract ProxyConfigLocalTestScript is Script {
    uint256 deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");
    address payable controllerContract = payable(vm.envAddress("CONTROLLER_ADDRESS"));
    uint256 nodeStakingAmount = vm.envUint("NODE_STAKING_AMOUNT");
    uint256 disqualifiedNodePenaltyAmount = vm.envUint("DISQUALIFIED_NODE_PENALTY_AMOUNT");
    uint256 defaultNumberOfCommitters = vm.envUint("DEFAULT_NUMBER_OF_COMMITTERS");
    uint256 defaultDkgPhaseDuration = vm.envUint("DEFAULT_DKG_PHASE_DURATION");
    uint256 groupMaxCapacity = vm.envUint("GROUP_MAX_CAPACITY");
    uint256 idealNumberOfGroups = vm.envUint("IDEAL_NUMBER_OF_GROUPS");
    uint256 pendingBlockAfterQuit = vm.envUint("PENDING_BLOCK_AFTER_QUIT");
    uint256 dkgPostProcessReward = vm.envUint("DKG_POST_PROCESS_REWARD");

    uint16 minimumRequestConfirmations = uint16(vm.envUint("MINIMUM_REQUEST_CONFIRMATIONS"));
    uint32 maxGasLimit = uint32(vm.envUint("MAX_GAS_LIMIT"));
    uint32 stalenessSeconds = uint32(vm.envUint("STALENESS_SECONDS"));
    uint32 gasAfterPaymentCalculation = uint32(vm.envUint("GAS_AFTER_PAYMENT_CALCULATION"));
    uint32 gasExceptCallback = uint32(vm.envUint("GAS_EXCEPT_CALLBACK"));
    int256 fallbackWeiPerUnitArpa = vm.envInt("FALLBACK_WEI_PER_UNIT_ARPA");
    uint256 signatureTaskExclusiveWindow = vm.envUint("SIGNATURE_TASK_EXCLUSIVE_WINDOW");
    uint256 rewardPerSignature = vm.envUint("REWARD_PER_SIGNATURE");
    uint256 committerRewardPerSignature = vm.envUint("COMMITTER_REWARD_PER_SIGNATURE");

    uint32 fulfillmentFlatFeeArpaPPMTier1 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER1"));
    uint32 fulfillmentFlatFeeArpaPPMTier2 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER2"));
    uint32 fulfillmentFlatFeeArpaPPMTier3 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER3"));
    uint32 fulfillmentFlatFeeArpaPPMTier4 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER4"));
    uint32 fulfillmentFlatFeeArpaPPMTier5 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER5"));
    uint24 reqsForTier2 = uint24(vm.envUint("REQS_FOR_TIER2"));
    uint24 reqsForTier3 = uint24(vm.envUint("REQS_FOR_TIER3"));
    uint24 reqsForTier4 = uint24(vm.envUint("REQS_FOR_TIER4"));
    uint24 reqsForTier5 = uint24(vm.envUint("REQS_FOR_TIER5"));

    function setUp() public {}

    function run() external {
        vm.broadcast(deployerPrivateKey);
        Proxy proxy = new Proxy(controllerContract);
        vm.broadcast(deployerPrivateKey);
        proxy.setControllerConfig(
            nodeStakingAmount,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            pendingBlockAfterQuit,
            dkgPostProcessReward
        );

        vm.broadcast(deployerPrivateKey);
        proxy.setAdapterConfig(
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
    }
}
