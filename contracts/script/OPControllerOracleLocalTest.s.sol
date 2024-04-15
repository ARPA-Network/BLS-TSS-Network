// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Deployer} from "./Deployer.s.sol";
import {ControllerOracle} from "../src/ControllerOracle.sol";
import {Adapter} from "../src/Adapter.sol";
import {IAdapterOwner} from "../src/interfaces/IAdapterOwner.sol";
import {Arpa} from "./ArpaLocalTest.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";

// solhint-disable-next-line max-states-count
contract OPControllerOracleLocalTestScript is Deployer {
    uint256 internal _deployerPrivateKey = vm.envUint("ADMIN_PRIVATE_KEY");

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

    bool internal _arpaExists = vm.envBool("ARPA_EXISTS");

    function run() external {
        ControllerOracle controllerOracle;
        ERC1967Proxy adapter;
        Adapter adapterImpl;

        _checkDeploymentAddressesFile();

        if (!_arpaExists) {
            IERC20 arpa;
            vm.broadcast(_deployerPrivateKey);
            arpa = new Arpa();
            _addDeploymentAddress(Network.L2, "Arpa", address(arpa));
        }

        vm.broadcast(_deployerPrivateKey);
        controllerOracle = new ControllerOracle();
        _addDeploymentAddress(Network.L2, "ControllerOracle", address(controllerOracle));

        vm.broadcast(_deployerPrivateKey);
        adapterImpl = new Adapter();
        _addDeploymentAddress(Network.L2, "AdapterImpl", address(adapterImpl));

        vm.broadcast(_deployerPrivateKey);
        adapter = new ERC1967Proxy(
            address(adapterImpl), abi.encodeWithSignature("initialize(address)", address(controllerOracle))
        );
        _addDeploymentAddress(Network.L2, "Adapter", address(adapter));

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
    }
}
