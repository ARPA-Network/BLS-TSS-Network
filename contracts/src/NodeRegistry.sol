// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IERC20, SafeERC20} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import {UUPSUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import {OwnableUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";
import {INodeRegistry, ISignatureUtils} from "./interfaces/INodeRegistry.sol";
import {INodeRegistryOwner} from "./interfaces/INodeRegistryOwner.sol";
import {IController} from "./interfaces/IController.sol";
import {INodeStaking} from "Staking-v0.1/interfaces/INodeStaking.sol";
import {IServiceManager} from "./interfaces/IServiceManager.sol";
import {BLS} from "./libraries/BLS.sol";

contract NodeRegistry is UUPSUpgradeable, INodeRegistry, INodeRegistryOwner, OwnableUpgradeable {
    using SafeERC20 for IERC20;

    // *Constants*
    uint16 private constant _BALANCE_BASE = 1;

    // *NodeRegistry Config*
    NodeRegistryConfig private _config;
    IERC20 private _arpa;

    // *Node State Variables*
    mapping(address => Node) private _nodes; // maps node address to Node Struct
    mapping(address => uint256) private _withdrawableEths; // maps node address to withdrawable eth amount
    mapping(address => uint256) private _arpaRewards; // maps node address to arpa rewards

    // *Events*
    event NodeRegistered(address indexed nodeAddress, bytes dkgPublicKey, uint256 groupIndex);
    event NodeActivated(address indexed nodeAddress, uint256 groupIndex);
    event NodeQuit(address indexed nodeAddress);
    event DkgPublicKeyChanged(address indexed nodeAddress, bytes dkgPublicKey);
    event NodeRewarded(address indexed nodeAddress, uint256 ethAmount, uint256 arpaAmount);
    event NodeSlashed(address indexed nodeIdAddress, uint256 stakingRewardPenalty, uint256 pendingBlock);

    // *Errors*
    error NodeNotRegistered();
    error NodeAlreadyRegistered();
    error NodeAlreadyActive();
    error NodeStillPending(uint256 pendingUntilBlock);
    error SenderNotController();
    error InvalidZeroAddress();
    error OperatorUnderStaking();

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(address arpa) public override(INodeRegistryOwner) initializer {
        _arpa = IERC20(arpa);

        __Ownable_init();
    }

    // solhint-disable-next-line no-empty-blocks
    function _authorizeUpgrade(address) internal override onlyOwner {}

    function setNodeRegistryConfig(
        address controllerContractAddress,
        address stakingContractAddress,
        address serviceManagerContractAddress,
        uint256 nativeNodeStakingAmount,
        uint256 eigenlayerNodeStakingAmount,
        uint256 pendingBlockAfterQuit
    ) external override(INodeRegistryOwner) onlyOwner {
        _config = NodeRegistryConfig(
            controllerContractAddress,
            stakingContractAddress,
            serviceManagerContractAddress,
            nativeNodeStakingAmount,
            eigenlayerNodeStakingAmount,
            pendingBlockAfterQuit
        );
    }

    function dismissNode(address nodeIdAddress, uint256 pendingBlock) external override(INodeRegistryOwner) onlyOwner {
        _nodeQuitHelper(nodeIdAddress, pendingBlock);
    }

    // =============
    // INodeRegistry
    // =============
    function nodeRegister(
        bytes calldata dkgPublicKey,
        bool isEigenlayerNode,
        ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature
    ) external override(INodeRegistry) {
        if (_nodes[msg.sender].idAddress != address(0)) {
            revert NodeAlreadyRegistered();
        }

        uint256[4] memory publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
        if (!BLS.isValidPublicKey(publicKey)) {
            revert BLS.InvalidPublicKey();
        }

        if (isEigenlayerNode) {
            uint256 share = IServiceManager(_config.serviceManagerContractAddress).getOperatorShare(msg.sender);
            if (share < _config.eigenlayerNodeStakingAmount) {
                revert OperatorUnderStaking();
            }
            IServiceManager(_config.serviceManagerContractAddress).registerOperator(msg.sender, operatorSignature);
        } else {
            // Lock staking amount in Staking contract
            INodeStaking(_config.stakingContractAddress).lock(msg.sender, _config.nativeNodeStakingAmount);
        }

        // Populate Node struct and insert into nodes
        Node storage n = _nodes[msg.sender];
        n.idAddress = msg.sender;
        n.dkgPublicKey = dkgPublicKey;
        n.state = true;
        n.isEigenlayerNode = isEigenlayerNode;

        // Initialize withdrawable eths and arpa rewards to save gas for adapter call
        _withdrawableEths[msg.sender] = _BALANCE_BASE;
        _arpaRewards[msg.sender] = _BALANCE_BASE;

        uint256 groupIndex = IController(_config.controllerContractAddress).nodeJoin(msg.sender);

        emit NodeRegistered(msg.sender, dkgPublicKey, groupIndex);
    }

    function nodeActivate(ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature)
        external
        override(INodeRegistry)
    {
        Node storage node = _nodes[msg.sender];
        if (node.idAddress != msg.sender) {
            revert NodeNotRegistered();
        }

        if (node.state) {
            revert NodeAlreadyActive();
        }

        if (node.pendingUntilBlock > block.number) {
            revert NodeStillPending(node.pendingUntilBlock);
        }

        if (node.isEigenlayerNode) {
            uint256 share = IServiceManager(_config.serviceManagerContractAddress).getOperatorShare(msg.sender);
            if (share < _config.eigenlayerNodeStakingAmount) {
                revert OperatorUnderStaking();
            }
            IServiceManager(_config.serviceManagerContractAddress).registerOperator(msg.sender, operatorSignature);
        } else {
            // lock up to staking amount in Staking contract
            uint256 lockedAmount = INodeStaking(_config.stakingContractAddress).getLockedAmount(msg.sender);
            if (lockedAmount < _config.nativeNodeStakingAmount) {
                INodeStaking(_config.stakingContractAddress).lock(
                    msg.sender, _config.nativeNodeStakingAmount - lockedAmount
                );
            }
        }

        node.state = true;

        uint256 groupIndex = IController(_config.controllerContractAddress).nodeJoin(msg.sender);

        emit NodeActivated(msg.sender, groupIndex);
    }

    function nodeQuit() external override(INodeRegistry) {
        _nodeQuitHelper(msg.sender, _config.pendingBlockAfterQuit);
    }

    function changeDkgPublicKey(bytes calldata dkgPublicKey) external override(INodeRegistry) {
        Node storage node = _nodes[msg.sender];
        if (node.idAddress != msg.sender) {
            revert NodeNotRegistered();
        }

        if (node.state) {
            revert NodeAlreadyActive();
        }

        uint256[4] memory publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
        if (!BLS.isValidPublicKey(publicKey)) {
            revert BLS.InvalidPublicKey();
        }

        node.dkgPublicKey = dkgPublicKey;

        emit DkgPublicKeyChanged(msg.sender, dkgPublicKey);
    }

    function nodeWithdraw(address recipient) external override(INodeRegistry) {
        if (recipient == address(0)) {
            revert InvalidZeroAddress();
        }
        uint256 ethAmount = _withdrawableEths[msg.sender];
        uint256 arpaAmount = _arpaRewards[msg.sender];
        if (arpaAmount > _BALANCE_BASE) {
            _arpaRewards[msg.sender] = _BALANCE_BASE;
            _arpa.safeTransfer(recipient, arpaAmount - _BALANCE_BASE);
        }
        if (ethAmount > _BALANCE_BASE) {
            _withdrawableEths[msg.sender] = _BALANCE_BASE;
            IController(_config.controllerContractAddress).nodeWithdrawETH(recipient, ethAmount - _BALANCE_BASE);
        }
    }

    function addReward(address[] memory nodes, uint256 ethAmount, uint256 arpaAmount) public override(INodeRegistry) {
        if (msg.sender != _config.controllerContractAddress) {
            revert SenderNotController();
        }

        for (uint256 i = 0; i < nodes.length; i++) {
            _withdrawableEths[nodes[i]] += ethAmount;
            _arpaRewards[nodes[i]] += arpaAmount;
            emit NodeRewarded(nodes[i], ethAmount, arpaAmount);
        }
    }

    // Give node staking reward penalty and freezeNode
    function slashNode(address nodeIdAddress, uint256 stakingRewardPenalty, uint256 pendingBlock)
        public
        override(INodeRegistry)
    {
        if (msg.sender != _config.controllerContractAddress) {
            revert SenderNotController();
        }

        Node storage node = _nodes[nodeIdAddress];

        if (node.isEigenlayerNode) {
            IServiceManager(_config.serviceManagerContractAddress).slashDelegationStaking(
                nodeIdAddress, stakingRewardPenalty
            );
        } else {
            // slash staking reward in Staking contract
            INodeStaking(_config.stakingContractAddress).slashDelegationReward(nodeIdAddress, stakingRewardPenalty);
        }

        _freezeNode(nodeIdAddress, pendingBlock);

        emit NodeSlashed(nodeIdAddress, stakingRewardPenalty, pendingBlock);
    }

    // =============
    // View
    // =============
    function getDKGPublicKey(address nodeAddress) public view override(INodeRegistry) returns (bytes memory) {
        return _nodes[nodeAddress].dkgPublicKey;
    }

    function getNode(address nodeAddress) public view override(INodeRegistry) returns (Node memory) {
        return _nodes[nodeAddress];
    }

    function getNodeWithdrawableTokens(address nodeAddress)
        public
        view
        override(INodeRegistry)
        returns (uint256, uint256)
    {
        return (
            _withdrawableEths[nodeAddress] == 0 ? 0 : (_withdrawableEths[nodeAddress] - _BALANCE_BASE),
            _arpaRewards[nodeAddress] == 0 ? 0 : (_arpaRewards[nodeAddress] - _BALANCE_BASE)
        );
    }

    function getNodeRegistryConfig()
        public
        view
        override(INodeRegistry)
        returns (
            address controllerContractAddress,
            address stakingContractAddress,
            address serviceManagerContractAddress,
            uint256 nativeNodeStakingAmount,
            uint256 eigenlayerNodeStakingAmount,
            uint256 pendingBlockAfterQuit
        )
    {
        return (
            _config.controllerContractAddress,
            _config.stakingContractAddress,
            _config.serviceManagerContractAddress,
            _config.nativeNodeStakingAmount,
            _config.eigenlayerNodeStakingAmount,
            _config.pendingBlockAfterQuit
        );
    }

    // =============
    // Internal
    // =============
    function _freezeNode(address nodeIdAddress, uint256 pendingBlock) internal {
        // set node state to false for frozen node
        _nodes[nodeIdAddress].state = false;

        uint256 currentBlock = block.number;
        // if the node is already pending, add the pending block to the current pending block
        if (_nodes[nodeIdAddress].pendingUntilBlock > currentBlock) {
            _nodes[nodeIdAddress].pendingUntilBlock += pendingBlock;
            // else set the pending block to the current block + pending block
        } else {
            _nodes[nodeIdAddress].pendingUntilBlock = currentBlock + pendingBlock;
        }
    }

    function _nodeQuitHelper(address nodeIdAddress, uint256 pendingBlock) internal {
        Node storage node = _nodes[nodeIdAddress];

        if (node.idAddress != nodeIdAddress) {
            revert NodeNotRegistered();
        }

        IController(_config.controllerContractAddress).nodeLeave(nodeIdAddress);

        _freezeNode(nodeIdAddress, pendingBlock);

        if (node.isEigenlayerNode) {
            IServiceManager(_config.serviceManagerContractAddress).deregisterOperator(nodeIdAddress);
        } else {
            // unlock staking amount in Staking contract
            INodeStaking(_config.stakingContractAddress).unlock(nodeIdAddress, _config.nativeNodeStakingAmount);
        }

        emit NodeQuit(nodeIdAddress);
    }
}
