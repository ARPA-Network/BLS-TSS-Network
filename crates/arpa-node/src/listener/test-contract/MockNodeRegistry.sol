// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface INodeRegistry {
    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool isEigenlayerNode;
        bool state;
        uint256 pendingUntilBlock;
    }
    
    function getNode(address nodeAddress) external view returns (Node memory);
}

contract MockNodeRegistry is INodeRegistry {
    mapping(address => Node) public nodes;

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
}