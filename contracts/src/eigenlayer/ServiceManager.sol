// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {UUPSUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import {OwnableUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";
import {ISignatureUtils, IAVSDirectory} from "../interfaces/IAVSDirectory.sol";
import {IDelegationManager} from "../interfaces/IDelegationManager.sol";
import {IServiceManager} from "../interfaces/IServiceManager.sol";

contract ServiceManager is UUPSUpgradeable, IServiceManager, OwnableUpgradeable {
    // *Constants*
    /// @notice Constant used as a divisor in calculating weights.
    uint256 public constant WEIGHTING_DIVISOR = 1e18;

    // *ServiceManager Config*
    address public nodeRegistryAddress;
    IAVSDirectory public avsDirectory;
    IDelegationManager public delegationManager;

    // *ServiceManager Variables*
    address[] public strategy;
    uint256[] public strategyWeights;
    mapping(address => bool) public whitelist;
    bool public whitelistEnabled;

    /// @notice when applied to a function, only allows the RegistryCoordinator to call it
    modifier onlyNodeRegistry() {
        if (msg.sender != nodeRegistryAddress) {
            revert SenderNotNodeRegistry();
        }
        _;
    }

    modifier whitelisted(address operator) {
        if (whitelistEnabled && !whitelist[operator]) {
            revert OperatorNotInWhitelist();
        }
        _;
    }

    // *Events*
    event OperatorSlashed(address indexed operator, uint256 stakingPenalty);

    // *Errors*
    error SenderNotNodeRegistry();
    error OperatorNotInWhitelist();
    error StrategyAndWeightsLengthMismatch();

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(address _nodeRegistryAddress, address _avsDirectory, address _delegationManager)
        public
        initializer
    {
        nodeRegistryAddress = _nodeRegistryAddress;
        avsDirectory = IAVSDirectory(_avsDirectory);
        delegationManager = IDelegationManager(_delegationManager);

        __Ownable_init();
    }

    // solhint-disable-next-line no-empty-blocks
    function _authorizeUpgrade(address) internal override onlyOwner {}

    /**
     * @notice Update the AVS Metadata URI
     */
    function updateAVSMetadataURI(string memory _metadataURI) public virtual onlyOwner {
        avsDirectory.updateAVSMetadataURI(_metadataURI);
    }

    /**
     * @notice Add to whitelist
     */
    function addToWhitelist(address[] calldata toAddAddresses) external onlyOwner {
        for (uint256 i = 0; i < toAddAddresses.length; i++) {
            whitelist[toAddAddresses[i]] = true;
        }
    }

    /**
     * @notice Remove from whitelist
     */
    function removeFromWhitelist(address[] calldata toRemoveAddresses) external onlyOwner {
        for (uint256 i = 0; i < toRemoveAddresses.length; i++) {
            delete whitelist[toRemoveAddresses[i]];
        }
    }

    /**
     * @notice Set the whitelistEnabled flag
     */
    function setWhitelistEnabled(bool _whitelistEnabled) external onlyOwner {
        whitelistEnabled = _whitelistEnabled;
    }

    /**
     * @notice Set the strategy and weights
     */
    function setStrategyAndWeights(address[] calldata _strategy, uint256[] calldata _strategyWeights)
        external
        onlyOwner
    {
        if (_strategy.length != _strategyWeights.length) {
            revert StrategyAndWeightsLengthMismatch();
        }
        strategy = _strategy;
        strategyWeights = _strategyWeights;
    }

    // =============
    // IEigenlayerCoordinator
    // =============
    function registerOperator(address operator, ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature)
        external
        onlyNodeRegistry
        whitelisted(operator)
    {
        avsDirectory.registerOperatorToAVS(operator, operatorSignature);
    }

    function deregisterOperator(address operator) external onlyNodeRegistry {
        avsDirectory.deregisterOperatorFromAVS(operator);
    }

    function slashDelegationStaking(address operator, uint256 amount) external onlyNodeRegistry {
        // TODO - implement slashing logic according to the eigenlayer slasher
        emit OperatorSlashed(operator, amount);
    }

    // =============
    // View
    // =============

    function getOperatorShare(address operator) external view returns (uint256 share) {
        for (uint256 i = 0; i < strategy.length; i++) {
            share += IDelegationManager(delegationManager).operatorShares(operator, strategy[i]) * strategyWeights[i]
                / WEIGHTING_DIVISOR;
        }
    }

    /**
     * @notice Returns the list of strategies that the AVS supports for restaking
     * @dev This function is intended to be called off-chain
     * @dev No guarantee is made on uniqueness of each element in the returned array.
     *    The off-chain service should do that validation separately
     */
    function getRestakeableStrategies() external view returns (address[] memory) {
        return strategy;
    }

    /**
     * @notice Returns the list of strategies that the operator has potentially restaked on the AVS
     * @param operator The address of the operator to get restaked strategies for
     * @dev This function is intended to be called off-chain
     * @dev No guarantee is made on whether the operator has shares for a strategy in a quorum or uniqueness
     *      of each element in the returned array. The off-chain service should do that validation separately
     */
    function getOperatorRestakedStrategies(address operator) external view returns (address[] memory) {
        address[] memory restakedStrategies = new address[](strategy.length);
        uint256 index = 0;
        for (uint256 i = 0; i < strategy.length; i++) {
            if (IDelegationManager(delegationManager).operatorShares(operator, strategy[i]) > 0) {
                restakedStrategies[index] = strategy[i];
                index++;
            }
        }
        return restakedStrategies;
    }

    // =============
    // Internal
    // =============
}
