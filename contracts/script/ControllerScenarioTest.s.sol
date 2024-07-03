// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.18;

import {Script} from "forge-std/Script.sol";
import {Controller} from "../src/Controller.sol";
import {ControllerProxy} from "../src/ControllerProxy.sol";
import {ControllerRelayer} from "../src/ControllerRelayer.sol";
import {OPChainMessenger} from "../src/OPChainMessenger.sol";
import {IControllerOwner} from "../src/interfaces/IControllerOwner.sol";
import {NodeRegistry} from "../src/NodeRegistry.sol";
import {INodeRegistryOwner} from "../src/interfaces/INodeRegistryOwner.sol";
import {Adapter} from "../src/Adapter.sol";
import {IAdapterOwner} from "../src/interfaces/IAdapterOwner.sol";
import {Arpa} from "./ArpaLocalTest.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import {ServiceManager} from "../src/eigenlayer/ServiceManager.sol";
import {Deployer} from "./Deployer.s.sol";

// solhint-disable-next-line max-states-count
contract ControllerScenarioTest is Deployer {
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
    uint256 internal _flatFeePromotionStartTimestamp = block.timestamp;
    uint256 internal _flatFeePromotionEndTimestamp = block.timestamp + 86400;

    uint256 internal _initialMaxPoolSize = vm.envUint("INITIAL_MAX_POOL_SIZE");
    uint256 internal _initialMaxCommunityStakeAmount = vm.envUint("INITIAL_MAX_COMMUNITY_STAKE_AMOUNT");
    uint256 internal _minCommunityStakeAmount = vm.envUint("MIN_COMMUNITY_STAKE_AMOUNT");
    uint256 internal _operatorStakeAmount = vm.envUint("OPERATOR_STAKE_AMOUNT");
    uint256 internal _eigenlayerOperatorStakeAmount = vm.envUint("EIGENLAYER_OPERATOR_STAKE_AMOUNT");
    uint256 internal _minInitialOperatorCount = vm.envUint("MIN_INITIAL_OPERATOR_COUNT");
    uint256 internal _minRewardDuration = vm.envUint("MIN_REWARD_DURATION");
    uint256 internal _delegationRateDenominator = vm.envUint("DELEGATION_RATE_DENOMINATOR");
    uint256 internal _unstakeFreezingDuration = vm.envUint("UNSTAKE_FREEZING_DURATION");

    uint256 internal _opChainId = vm.envUint("OP_CHAIN_ID");
    address internal _opControllerOracleAddress = vm.envAddress("OP_CONTROLLER_ORACLE_ADDRESS");
    address internal _opL1CrossDomainMessengerAddress = vm.envAddress("OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS");

    address internal _avsDirectory = vm.envAddress("AVS_DIRECTORY_ADDRESS");
    address internal _delegationManager = vm.envAddress("DELEGATION_MANAGER_ADDRESS");

    function run() external {
        NodeRegistry nodeRegistryImpl;
        ERC1967Proxy nodeRegistry;
        Controller controllerImpl;
        Adapter adapterImpl;
        ERC1967Proxy adapter;
        ServiceManager serviceManagerImpl;
        ERC1967Proxy serviceManager;
        ControllerRelayer controllerRelayerImpl;
        ERC1967Proxy controllerRelayer;
        ControllerProxy controllerProxy;
        Staking staking;
        IERC20 arpa;
        _checkDeploymentAddressesFile();
        vm.broadcast(_deployerPrivateKey);
        arpa = new Arpa();
        _addDeploymentAddress(Network.L1, "Arpa", address(arpa));

        vm.broadcast(_deployerPrivateKey);
        nodeRegistryImpl = new NodeRegistry();
        _addDeploymentAddress(Network.L1, "NodeRegistryImpl", address(nodeRegistryImpl));

        vm.broadcast(_deployerPrivateKey);
        nodeRegistry =
            new ERC1967Proxy(address(nodeRegistryImpl), abi.encodeWithSignature("initialize(address)", address(arpa)));
        _addDeploymentAddress(Network.L1, "NodeRegistry", address(nodeRegistry));

        vm.broadcast(_deployerPrivateKey);
        serviceManagerImpl = new ServiceManager();
        _addDeploymentAddress(Network.L1, "ServiceManagerImpl", address(serviceManagerImpl));

        vm.broadcast(_deployerPrivateKey);
        serviceManager = new ERC1967Proxy(
            address(serviceManagerImpl),
            abi.encodeWithSignature(
                "initialize(address,address,address)", address(nodeRegistry), _avsDirectory, _delegationManager
            )
        );
        _addDeploymentAddress(Network.L1, "ServiceManager", address(serviceManager));

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
        _addDeploymentAddress(Network.L1, "Staking", address(staking));

        vm.broadcast(_deployerPrivateKey);
        staking.setController(address(nodeRegistry));

        vm.broadcast(_deployerPrivateKey);
        controllerImpl = new Controller();
        _addDeploymentAddress(Network.L1, "ControllerImpl", address(controllerImpl));

        vm.broadcast(_deployerPrivateKey);
        controllerProxy =
            new ControllerProxy(address(controllerImpl), abi.encodeWithSignature("initialize(uint256)", _lastOutput));
        _addDeploymentAddress(Network.L1, "ControllerProxy", address(controllerProxy));

        vm.broadcast(_deployerPrivateKey);
        INodeRegistryOwner(address(nodeRegistry)).setNodeRegistryConfig(
            address(controllerProxy),
            address(staking),
            address(serviceManager),
            _operatorStakeAmount,
            _eigenlayerOperatorStakeAmount,
            _pendingBlockAfterQuit
        );

        vm.broadcast(_deployerPrivateKey);
        adapterImpl = new Adapter();
        _addDeploymentAddress(Network.L1, "AdapterImpl", address(adapterImpl));

        vm.broadcast(_deployerPrivateKey);
        adapter = new ERC1967Proxy(address(adapterImpl), abi.encodeWithSignature("initialize(address)", address(controllerProxy)));
        _addDeploymentAddress(Network.L1, "Adapter", address(adapter));


        vm.broadcast(_deployerPrivateKey);
        IControllerOwner(address(controllerProxy)).setControllerConfig(
            address(nodeRegistry),
            address(adapter),
            _disqualifiedNodePenaltyAmount,
            _defaultNumberOfCommitters,
            _defaultDkgPhaseDuration,
            _groupMaxCapacity,
            _idealNumberOfGroups,
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
        controllerRelayerImpl = new ControllerRelayer();
        _addDeploymentAddress(Network.L1, "ControllerRelayerImpl", address(controllerRelayerImpl));

        vm.broadcast(_deployerPrivateKey);
        controllerRelayer = new ERC1967Proxy(
            address(controllerRelayerImpl), abi.encodeWithSignature("initialize(address)", address(controllerProxy))
        );
        _addDeploymentAddress(Network.L1, "ControllerRelayer", address(controllerRelayer));
    }
}
