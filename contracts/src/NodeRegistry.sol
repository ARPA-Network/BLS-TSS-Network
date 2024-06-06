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
import {IERC1271} from "openzeppelin-contracts/contracts/interfaces/IERC1271.sol";
import {Address} from "openzeppelin-contracts/contracts/utils/Address.sol";
import {ECDSA} from "openzeppelin-contracts/contracts/utils/cryptography/ECDSA.sol";

contract NodeRegistry is UUPSUpgradeable, INodeRegistry, INodeRegistryOwner, OwnableUpgradeable {
    using SafeERC20 for IERC20;

    // *Constants*
    /// @notice The EIP-712 typehash for the contract's domain
    bytes32 public constant DOMAIN_TYPEHASH =
        keccak256("EIP712Domain(string name,uint256 chainId,address verifyingContract)");
    /// @notice The EIP-712 typehash for the `Registration` struct used by the contract
    bytes32 public constant NATIVE_NODE_REGISTRATION_TYPEHASH =
        keccak256("NativeNodeRegistration(address assetAccountAddress,bytes32 salt,uint256 expiry)");
    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 internal constant _EIP1271_MAGICVALUE = 0x1626ba7e;
    uint16 private constant _BALANCE_BASE = 1;

    // *NodeRegistry Config*
    NodeRegistryConfig private _config;
    IERC20 private _arpa;

    // *Node State Variables*
    mapping(address => Node) private _nodes; // maps node address to Node Struct
    mapping(address => uint256) private _withdrawableEths; // maps node address to withdrawable eth amount
    mapping(address => uint256) private _arpaRewards; // maps node address to arpa rewards
    mapping(address => address) private _assetAccountsToNodes; // maps asset account address to node address
    mapping(address => address) private _nodesToAssetAccounts; // maps node address to asset account address
    mapping(address => mapping(bytes32 => bool)) private _assetAccountSaltIsSpent; // maps asset account address to salt

    // *Events*
    event NodeRegistered(address indexed nodeAddress, bytes dkgPublicKey, uint256 groupIndex);
    event NodeActivated(address indexed nodeAddress, uint256 groupIndex);
    event NodeQuit(address indexed nodeAddress);
    event DkgPublicKeyChanged(address indexed nodeAddress, bytes dkgPublicKey);
    event NodeRewarded(address indexed nodeAddress, uint256 ethAmount, uint256 arpaAmount);
    event NodeSlashed(address indexed nodeIdAddress, uint256 stakingRewardPenalty, uint256 pendingBlock);
    event AssetAccountSet(address indexed assetAccountAddress, address indexed nodeAddress);

    // *Errors*
    error NodeNotRegistered();
    error NodeAlreadyRegistered();
    error NodeAlreadyActive();
    error NodeStillPending(uint256 pendingUntilBlock);
    error SenderNotController();
    error InvalidZeroAddress();
    error OperatorUnderStaking();
    error EIP1271SignatureVerificationFailed();
    error EIP1271SignatureNotFromSigner();
    error EIP1271SignatureExpired();
    error EIP1271SignatureSaltAlreadySpent();
    error InvalidArrayLength();

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

    function setAssetAccount(address[] calldata assetAccountAddresses, address[] calldata nodeAddresses)
        external
        override(INodeRegistryOwner)
        onlyOwner
    {
        if (assetAccountAddresses.length != nodeAddresses.length) {
            revert InvalidArrayLength();
        }
        for (uint256 i = 0; i < assetAccountAddresses.length; i++) {
            _assetAccountsToNodes[assetAccountAddresses[i]] = nodeAddresses[i];
            _nodesToAssetAccounts[nodeAddresses[i]] = assetAccountAddresses[i];
            emit AssetAccountSet(assetAccountAddresses[i], nodeAddresses[i]);
        }
    }

    // =============
    // INodeRegistry
    // =============
    function nodeRegister(
        bytes calldata dkgPublicKey,
        bool isEigenlayerNode,
        address assetAccountAddress,
        ISignatureUtils.SignatureWithSaltAndExpiry memory assetAccountSignature
    ) external override(INodeRegistry) {
        if (_assetAccountsToNodes[assetAccountAddress] != address(0)) {
            revert NodeAlreadyRegistered();
        }

        _nodeRegister(dkgPublicKey, isEigenlayerNode);

        _assetAccountsToNodes[assetAccountAddress] = msg.sender;
        _nodesToAssetAccounts[msg.sender] = assetAccountAddress;

        if (isEigenlayerNode) {
            uint256 share = IServiceManager(_config.serviceManagerContractAddress).getOperatorShare(assetAccountAddress);
            if (share < _config.eigenlayerNodeStakingAmount) {
                revert OperatorUnderStaking();
            }
            IServiceManager(_config.serviceManagerContractAddress).registerOperator(
                assetAccountAddress, assetAccountSignature
            );
        } else {
            if (msg.sender != assetAccountAddress) {
                _checkEIP1271SignatureWithSaltAndExpiry(assetAccountAddress, assetAccountSignature);
            }
            // Lock staking amount in Staking contract
            INodeStaking(_config.stakingContractAddress).lock(assetAccountAddress, _config.nativeNodeStakingAmount);
        }

        emit AssetAccountSet(assetAccountAddress, msg.sender);
    }

    function nodeActivate(ISignatureUtils.SignatureWithSaltAndExpiry memory assetAccountSignature)
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

        node.state = true;

        uint256 groupIndex = IController(_config.controllerContractAddress).nodeJoin(msg.sender);

        emit NodeActivated(msg.sender, groupIndex);

        address assetAccountAddress = _nodesToAssetAccounts[msg.sender];

        if (node.isEigenlayerNode) {
            uint256 share = IServiceManager(_config.serviceManagerContractAddress).getOperatorShare(assetAccountAddress);
            if (share < _config.eigenlayerNodeStakingAmount) {
                revert OperatorUnderStaking();
            }
            IServiceManager(_config.serviceManagerContractAddress).registerOperator(
                assetAccountAddress, assetAccountSignature
            );
        } else {
            if (msg.sender != assetAccountAddress) {
                _checkEIP1271SignatureWithSaltAndExpiry(assetAccountAddress, assetAccountSignature);
            }
            // lock up to staking amount in Staking contract
            uint256 lockedAmount = INodeStaking(_config.stakingContractAddress).getLockedAmount(assetAccountAddress);
            if (lockedAmount < _config.nativeNodeStakingAmount) {
                INodeStaking(_config.stakingContractAddress).lock(
                    assetAccountAddress, _config.nativeNodeStakingAmount - lockedAmount
                );
            }
        }
    }

    function nodeQuit() external override(INodeRegistry) {
        _nodeQuitHelper(msg.sender, _config.pendingBlockAfterQuit);
    }

    function nodeLogOff() external override(INodeRegistry) {
        address nodeAccountAddress = _assetAccountsToNodes[msg.sender];
        if (nodeAccountAddress == address(0)) {
            revert NodeNotRegistered();
        }
        if (_nodes[nodeAccountAddress].state) {
            revert NodeAlreadyActive();
        }
        delete _assetAccountsToNodes[msg.sender];
        delete _nodesToAssetAccounts[nodeAccountAddress];

        emit AssetAccountSet(msg.sender, address(0));
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

        address assetAccountAddress = _nodesToAssetAccounts[nodeIdAddress];

        if (node.isEigenlayerNode) {
            IServiceManager(_config.serviceManagerContractAddress).slashDelegationStaking(
                assetAccountAddress, stakingRewardPenalty
            );
            IServiceManager(_config.serviceManagerContractAddress).deregisterOperator(assetAccountAddress);
        } else {
            // slash staking reward in Staking contract
            INodeStaking(_config.stakingContractAddress).slashDelegationReward(
                assetAccountAddress, stakingRewardPenalty
            );
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

    function getNodeAddressByAssetAccountAddress(address assetAccountAddress)
        public
        view
        override(INodeRegistry)
        returns (address)
    {
        return _assetAccountsToNodes[assetAccountAddress];
    }

    function getAssetAccountAddressByNodeAddress(address nodeAddress)
        public
        view
        override(INodeRegistry)
        returns (address)
    {
        return _nodesToAssetAccounts[nodeAddress];
    }

    /**
     * @notice Calculates the digest hash to be signed as a native node
     * @param assetAccountAddress The asset account address of the staking node
     * @param salt A unique and single use value associated with the approver signature.
     * @param expiry Time after which the approver's signature becomes invalid
     */
    function calculateNativeNodeRegistrationDigestHash(address assetAccountAddress, bytes32 salt, uint256 expiry)
        public
        view
        override(INodeRegistry)
        returns (bytes32)
    {
        // calculate the struct hash
        bytes32 structHash = keccak256(abi.encode(NATIVE_NODE_REGISTRATION_TYPEHASH, assetAccountAddress, salt, expiry));
        // calculate the digest hash
        bytes32 digestHash = keccak256(abi.encodePacked("\x19\x01", domainSeparator(), structHash));
        return digestHash;
    }

    /**
     * @notice Getter function for the current EIP-712 domain separator for this contract.
     */
    function domainSeparator() public view override(INodeRegistry) returns (bytes32) {
        return keccak256(abi.encode(DOMAIN_TYPEHASH, keccak256(bytes("ARPANetwork")), block.chainid, address(this)));
    }

    function assetAccountSaltIsSpent(address assetAccountAddress, bytes32 salt)
        public
        view
        override(INodeRegistry)
        returns (bool)
    {
        return _assetAccountSaltIsSpent[assetAccountAddress][salt];
    }

    // =============
    // Internal
    // =============
    function _nodeRegister(bytes calldata dkgPublicKey, bool isEigenlayerNode) internal {
        if (_nodes[msg.sender].idAddress != address(0)) {
            revert NodeAlreadyRegistered();
        }

        uint256[4] memory publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
        if (!BLS.isValidPublicKey(publicKey)) {
            revert BLS.InvalidPublicKey();
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

        address assetAccountAddress = _nodesToAssetAccounts[nodeIdAddress];

        if (node.isEigenlayerNode) {
            IServiceManager(_config.serviceManagerContractAddress).deregisterOperator(assetAccountAddress);
        } else {
            // unlock staking amount in Staking contract
            INodeStaking(_config.stakingContractAddress).unlock(assetAccountAddress, _config.nativeNodeStakingAmount);
        }

        emit NodeQuit(nodeIdAddress);
    }

    function _checkEIP1271SignatureWithSaltAndExpiry(
        address assetAccountAddress,
        ISignatureUtils.SignatureWithSaltAndExpiry memory assetAccountSignature
    ) internal {
        if (assetAccountSignature.expiry < block.timestamp) {
            revert EIP1271SignatureExpired();
        }
        if (_assetAccountSaltIsSpent[assetAccountAddress][assetAccountSignature.salt]) {
            revert EIP1271SignatureSaltAlreadySpent();
        }
        bytes32 nativeNodeRegistrationDigestHash = calculateNativeNodeRegistrationDigestHash(
            assetAccountAddress, assetAccountSignature.salt, assetAccountSignature.expiry
        );
        _checkEIP1271Signature(assetAccountAddress, nativeNodeRegistrationDigestHash, assetAccountSignature.signature);
        _assetAccountSaltIsSpent[assetAccountAddress][assetAccountSignature.salt] = true;
    }

    /**
     * @notice Checks @param signature is a valid signature of @param digestHash from @param signer.
     * If the `signer` contains no code -- i.e. it is not (yet, at least) a contract address, then checks using standard ECDSA logic
     * Otherwise, passes on the signature to the signer to verify the signature and checks that it returns the `EIP1271_MAGICVALUE`.
     */
    function _checkEIP1271Signature(address signer, bytes32 digestHash, bytes memory signature) internal view {
        /**
         * check validity of signature:
         * 1) if `signer` is an EOA, then `signature` must be a valid ECDSA signature from `signer`,
         * indicating their intention for this action
         * 2) if `signer` is a contract, then `signature` must will be checked according to EIP-1271
         */
        if (Address.isContract(signer)) {
            if (IERC1271(signer).isValidSignature(digestHash, signature) != _EIP1271_MAGICVALUE) {
                revert EIP1271SignatureVerificationFailed();
            }
        } else {
            if (ECDSA.recover(digestHash, signature) != signer) {
                revert EIP1271SignatureNotFromSigner();
            }
        }
    }
}
