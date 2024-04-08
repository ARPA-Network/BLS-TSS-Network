// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {OwnableUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/Initializable.sol";
import {ISignatureUtils, IAVSDirectory} from "./interfaces/IAVSDirectory.sol";
import {IDelegationManager} from "./interfaces/IDelegationManager.sol";

contract EigenlayerCoordinator is Initializable, OwnableUpgradeable {
    // *Constants*

    // *NodeRegistry Config*
    address public nodeRegistryAddress;
    address public stETHStrategyAddress;
    IAVSDirectory public avsDirectory;
    IDelegationManager public delegationManager;

    // *Node State Variables*

    /// @notice when applied to a function, only allows the RegistryCoordinator to call it
    modifier onlyNodeRegistry() {
        if (msg.sender != nodeRegistryAddress) {
            revert SenderNotNodeRegistry();
        }
        _;
    }

    // *Events*
    event OperatorSlashed(address indexed operator, uint256 stakingPenalty);

    // *Errors*
    error SenderNotNodeRegistry();

    function initialize(
        address _nodeRegistryAddress,
        address _stETHStrategyAddress,
        address _avsDirectory,
        address _delegationManager
    ) public initializer {
        nodeRegistryAddress = _nodeRegistryAddress;
        stETHStrategyAddress = _stETHStrategyAddress;
        avsDirectory = IAVSDirectory(_avsDirectory);
        delegationManager = IDelegationManager(_delegationManager);

        __Ownable_init();
    }

    function updateAVSMetadataURI(string memory _metadataURI) public virtual onlyOwner {
        avsDirectory.updateAVSMetadataURI(_metadataURI);
    }

    // =============
    // IEigenlayerCoordinator
    // =============
    function registerOperator(address operator, ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature)
        external
        onlyNodeRegistry
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

    function getOperatorShare(address operator) external view returns (uint256) {
        return delegationManager.operatorShares(operator, stETHStrategyAddress);
    }

    // =============
    // Internal
    // =============
}
