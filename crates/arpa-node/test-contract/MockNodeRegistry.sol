// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MockNodeRegistry {
    mapping(address => Node) public nodes;

    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool isEigenlayerNode;
        bool state;
        uint256 pendingUntilBlock;
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

    function getNode(address nodeAddress) public view returns (Node memory) {
        return nodes[nodeAddress];
    }
}
