// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface ISignatureUtils {
    struct SignatureWithSaltAndExpiry {
        bytes signature;
        bytes32 salt;
        uint256 expiry;
    }
}

interface INodeRegistry {
    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool isEigenlayerNode;
        bool state;
        uint256 pendingUntilBlock;
    }
    
    function getNode(address nodeAddress) external view returns (Node memory);
    function nodeActivate(ISignatureUtils.SignatureWithSaltAndExpiry memory assetAccountSignature) external;
    function getNodeRegistryConfig() external view returns (
        address controllerContractAddress,
        address stakingContractAddress,
        address serviceManagerContractAddress,
        uint256 nativeNodeStakingAmount,
        uint256 eigenlayerNodeStakingAmount,
        uint256 pendingBlockAfterQuit
    );
}

contract MockAVSDirectory {
    bytes32 public constant OPERATOR_AVS_REGISTRATION_TYPEHASH = 
        keccak256("OperatorAVSRegistration(address operator,address avs,bytes32 salt,uint256 expiry)");
    
    bytes32 private constant _TYPE_HASH = 
        keccak256("EIP712Domain(string name,uint256 chainId,address verifyingContract)");
    
    function calculateOperatorAVSRegistrationDigestHash(
        address operator,
        address avs,
        bytes32 salt,
        uint256 expiry
    ) public view returns (bytes32) {
        return _calculateSignableDigest(
            keccak256(abi.encode(OPERATOR_AVS_REGISTRATION_TYPEHASH, operator, avs, salt, expiry))
        );
    }
    
    function _calculateSignableDigest(bytes32 structHash) internal view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", _domainSeparator(), structHash));
    }
    
    function _domainSeparator() internal view returns (bytes32) {
        return keccak256(abi.encode(
            _TYPE_HASH,
            keccak256("MockAVSDirectory"),
            block.chainid,
            address(this)
        ));
    }
}

contract MockServiceManager {
    address public avsDirectory;
    
    constructor(address _avsDirectory) {
        avsDirectory = _avsDirectory;
    }
}

contract MockNodeRegistry is INodeRegistry {
    struct Config {
        address controllerContractAddress;
        address stakingContractAddress;
        address serviceManagerContractAddress;
        uint256 nativeNodeStakingAmount;
        uint256 eigenlayerNodeStakingAmount;
        uint256 pendingBlockAfterQuit;
    }
    
    mapping(address => Node) public nodes;
    Config private _config;
    
    event NodeActivated(address indexed nodeAddress, uint256 groupIndex);
    
    error NodeNotRegistered();
    error NodeAlreadyActive();
    error NodeStillPending(uint256 pendingUntilBlock);

    constructor(
        address _controllerAddress,
        address _stakingAddress,
        address _serviceManagerAddress
    ) {
        _config = Config({
            controllerContractAddress: _controllerAddress,
            stakingContractAddress: _stakingAddress,
            serviceManagerContractAddress: _serviceManagerAddress,
            nativeNodeStakingAmount: 100 ether,
            eigenlayerNodeStakingAmount: 1000 ether,
            pendingBlockAfterQuit: 1000
        });
    }

    function registerNode(address idAddress, bytes memory dkgPublicKey, bool isEigenlayerNode) external {
        nodes[idAddress] = Node({
            idAddress: idAddress,
            dkgPublicKey: dkgPublicKey,
            isEigenlayerNode: isEigenlayerNode,
            state: false,
            pendingUntilBlock: 0
        });
    }

    function setNodeState(address idAddress, bool state) external {
        nodes[idAddress].state = state;
    }

    function getNode(address nodeAddress) public view override(INodeRegistry) returns (Node memory) {
        return nodes[nodeAddress];
    }
    
    function nodeActivate(ISignatureUtils.SignatureWithSaltAndExpiry memory assetAccountSignature)
        external
        override(INodeRegistry)
    {
        Node storage node = nodes[msg.sender];
        
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

        uint256 groupIndex = 1;

        emit NodeActivated(msg.sender, groupIndex);
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
}