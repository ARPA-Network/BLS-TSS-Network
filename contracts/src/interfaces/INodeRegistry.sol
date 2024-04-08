// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {ISignatureUtils} from "./ISignatureUtils.sol";

interface INodeRegistry {
    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool state;
        uint256 pendingUntilBlock;
    }

    struct NodeRegistryConfig {
        address controllerContractAddress;
        address stakingContractAddress;
        uint256 nodeStakingAmount;
        uint256 pendingBlockAfterQuit;
    }

    // node transaction
    function nodeRegister(
        bytes calldata dkgPublicKey,
        ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature
    ) external;

    function nodeActivate() external;

    function nodeQuit() external;

    function changeDkgPublicKey(bytes calldata dkgPublicKey) external;

    function nodeWithdraw(address recipient) external;

    // controller transaction
    function slashNode(address nodeIdAddress, uint256 stakingRewardPenalty, uint256 pendingBlock) external;

    // adapter transaction
    function addReward(address[] memory nodes, uint256 ethAmount, uint256 arpaAmount) external;

    // view
    function getDKGPublicKey(address nodeAddress) external view returns (bytes memory);

    function getNode(address nodeAddress) external view returns (Node memory);

    function getNodeWithdrawableTokens(address nodeAddress) external view returns (uint256, uint256);

    function getNodeRegistryConfig()
        external
        view
        returns (
            address controllerContractAddress,
            address stakingContractAddress,
            uint256 nodeStakingAmount,
            uint256 pendingBlockAfterQuit
        );

    function isDeployedOnEigenlayer() external view returns (bool);
}
