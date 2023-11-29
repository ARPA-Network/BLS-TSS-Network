// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {Controller} from "../src/Controller.sol";
import {ControllerRelayer} from "../src/ControllerRelayer.sol";
import {OPChainMessenger} from "../src/OPChainMessenger.sol";
import {IControllerOwner} from "../src/interfaces/IControllerOwner.sol";
import {Adapter} from "../src/Adapter.sol";
import {IAdapterOwner} from "../src/interfaces/IAdapterOwner.sol";
import {Arpa} from "./ArpaLocalTest.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {Staking} from "Staking-v0.1/Staking.sol";

// solhint-disable-next-line max-states-count
contract ControllerLocalTestScript is Script {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

    uint256 internal _disqualifiedNodePenaltyAmount = vm.envUint("DISQUALIFIED_NODE_PENALTY_AMOUNT");
    uint256 internal _defaultNumberOfCommitters = vm.envUint("DEFAULT_NUMBER_OF_COMMITTERS");
    uint256 internal _defaultDkgPhaseDuration = vm.envUint("DEFAULT_DKG_PHASE_DURATION");
    uint256 internal _groupMaxCapacity = vm.envUint("GROUP_MAX_CAPACITY");
    uint256 internal _idealNumberOfGroups = vm.envUint("IDEAL_NUMBER_OF_GROUPS");
    uint256 internal _pendingBlockAfterQuit = vm.envUint("PENDING_BLOCK_AFTER_QUIT");
    uint256 internal _dkgPostProcessReward = vm.envUint("DKG_POST_PROCESS_REWARD");
    uint256 internal _lastOutput = vm.envUint("LAST_OUTPUT");

    uint16 internal _minimumRequestConfirmations = uint16(vm.envUint("MINIMUM_REQUEST_CONFIRMATIONS"));
    uint32 internal _maxGasLimit = uint32(vm.envUint("MAX_GAS_LIMIT"));
    uint32 internal _gasAfterPaymentCalculation = uint32(vm.envUint("GAS_AFTER_PAYMENT_CALCULATION"));
    uint32 internal _gasExceptCallback = uint32(vm.envUint("GAS_EXCEPT_CALLBACK"));
    uint256 internal _signatureTaskExclusiveWindow = vm.envUint("SIGNATURE_TASK_EXCLUSIVE_WINDOW");
    uint256 internal _rewardPerSignature = vm.envUint("REWARD_PER_SIGNATURE");
    uint256 internal _committerRewardPerSignature = vm.envUint("COMMITTER_REWARD_PER_SIGNATURE");

    uint32 internal _fulfillmentFlatFeeEthPPMTier1 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER1"));
    uint32 internal _fulfillmentFlatFeeEthPPMTier2 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER2"));
    uint32 internal _fulfillmentFlatFeeEthPPMTier3 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER3"));
    uint32 internal _fulfillmentFlatFeeEthPPMTier4 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER4"));
    uint32 internal _fulfillmentFlatFeeEthPPMTier5 = uint32(vm.envUint("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER5"));
    uint24 internal _reqsForTier2 = uint24(vm.envUint("REQS_FOR_TIER2"));
    uint24 internal _reqsForTier3 = uint24(vm.envUint("REQS_FOR_TIER3"));
    uint24 internal _reqsForTier4 = uint24(vm.envUint("REQS_FOR_TIER4"));
    uint24 internal _reqsForTier5 = uint24(vm.envUint("REQS_FOR_TIER5"));

    uint16 internal _flatFeePromotionGlobalPercentage = uint16(vm.envUint("FLAT_FEE_PROMOTION_GLOBAL_PERCENTAGE"));
    bool internal _isFlatFeePromotionEnabledPermanently = vm.envBool("IS_FLAT_FEE_PROMOTION_ENABLED_PERMANENTLY");
    uint256 internal _flatFeePromotionStartTimestamp = vm.envUint("FLAT_FEE_PROMOTION_START_TIMESTAMP");
    uint256 internal _flatFeePromotionEndTimestamp = vm.envUint("FLAT_FEE_PROMOTION_END_TIMESTAMP");

    uint256 internal _initialMaxPoolSize = vm.envUint("INITIAL_MAX_POOL_SIZE");
    uint256 internal _initialMaxCommunityStakeAmount = vm.envUint("INITIAL_MAX_COMMUNITY_STAKE_AMOUNT");
    uint256 internal _minCommunityStakeAmount = vm.envUint("MIN_COMMUNITY_STAKE_AMOUNT");
    uint256 internal _operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");
    uint256 internal _minInitialOperatorCount = vm.envUint("MIN_INITIAL_OPERATOR_COUNT");
    uint256 internal _minRewardDuration = vm.envUint("MIN_REWARD_DURATION");
    uint256 internal _delegationRateDenominator = vm.envUint("DELEGATION_RATE_DENOMINATOR");
    uint256 internal _unstakeFreezingDuration = vm.envUint("UNSTAKE_FREEZING_DURATION");

    uint256 internal _opChainId = vm.envUint("OP_CHAIN_ID");
    address internal _opControllerOracleAddress = vm.envAddress("OP_CONTROLLER_ORACLE_ADDRESS");
    address internal _opL1CrossDomainMessengerAddress = vm.envAddress("OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS");

    bool internal _arpa_exists = vm.envBool("ARPA_EXISTS");
    address internal _existing_arpa_address = vm.envAddress("EXISTING_L1_ARPA_ADDRESS");

    function run() external {
        Controller controller;
        ControllerRelayer controllerRelayer;
        OPChainMessenger opChainMessenger;
        ERC1967Proxy adapter;
        Adapter adapterImpl;
        Staking staking;
        IERC20 arpa;

        if (_arpa_exists == false) {
            vm.broadcast(_deployerPrivateKey);
            arpa = new Arpa();
        } else {
            arpa = IERC20(_existing_arpa_address);
        }

        Staking.PoolConstructorParams memory params = Staking.PoolConstructorParams(
            IERC20(address(arpa)),
            _initialMaxPoolSize,
            _initialMaxCommunityStakeAmount,
            _minCommunityStakeAmount,
            _operatorStakeAmount,
            _minInitialOperatorCount,
            _minRewardDuration,
            _delegationRateDenominator,
            _unstakeFreezingDuration
        );
        vm.broadcast(_deployerPrivateKey);
        staking = new Staking(params);

        vm.broadcast(_deployerPrivateKey);
        controller = new Controller();

        vm.broadcast(_deployerPrivateKey);
        controller.initialize(address(staking), _lastOutput);

        vm.broadcast(_deployerPrivateKey);
        adapterImpl = new Adapter();

        vm.broadcast(_deployerPrivateKey);
        adapter =
            new ERC1967Proxy(address(adapterImpl),abi.encodeWithSignature("initialize(address)",address(controller)));

        vm.broadcast(_deployerPrivateKey);
        IControllerOwner(address(controller)).setControllerConfig(
            address(staking),
            address(adapter),
            _operatorStakeAmount,
            _disqualifiedNodePenaltyAmount,
            _defaultNumberOfCommitters,
            _defaultDkgPhaseDuration,
            _groupMaxCapacity,
            _idealNumberOfGroups,
            _pendingBlockAfterQuit,
            _dkgPostProcessReward
        );

        vm.broadcast(_deployerPrivateKey);
        IAdapterOwner(address(adapter)).setAdapterConfig(
            _minimumRequestConfirmations,
            _maxGasLimit,
            _gasAfterPaymentCalculation,
            _gasExceptCallback,
            _signatureTaskExclusiveWindow,
            _rewardPerSignature,
            _committerRewardPerSignature
        );

        vm.broadcast(_deployerPrivateKey);
        IAdapterOwner(address(adapter)).setFlatFeeConfig(
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

        vm.broadcast(_deployerPrivateKey);
        staking.setController(address(controller));

        vm.broadcast(_deployerPrivateKey);
        controllerRelayer = new ControllerRelayer(address(controller));

        vm.broadcast(_deployerPrivateKey);
        opChainMessenger =
        new OPChainMessenger(address(controllerRelayer), _opControllerOracleAddress, _opL1CrossDomainMessengerAddress);

        vm.broadcast(_deployerPrivateKey);
        controllerRelayer.setChainMessenger(_opChainId, address(opChainMessenger));
    }
}
