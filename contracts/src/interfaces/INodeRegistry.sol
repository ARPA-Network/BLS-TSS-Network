// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {ISignatureUtils} from "./ISignatureUtils.sol";

interface INodeRegistry {
    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool isEigenlayerNode;
        bool state;
        uint256 pendingUntilBlock;
    }

    struct NodeRegistryConfig {
        address controllerContractAddress;
        address stakingContractAddress;
        address serviceManagerContractAddress;
        uint256 nativeNodeStakingAmount;
        uint256 eigenlayerNodeStakingAmount;
        uint256 pendingBlockAfterQuit;
    }

    // node transaction
    function nodeRegister(
        bytes calldata dkgPublicKey,
        bool isEigenlayerNode,
        address assetAccountAddress,
        ISignatureUtils.SignatureWithSaltAndExpiry memory assetAccountSignature
    ) external;

    function nodeActivate(ISignatureUtils.SignatureWithSaltAndExpiry memory assetAccountSignature) external;

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
            address serviceManagerContractAddress,
            uint256 nativeNodeStakingAmount,
            uint256 eigenlayerNodeStakingAmount,
            uint256 pendingBlockAfterQuit
        );

    function getNodeAddressByAssetAccountAddress(address assetAccountAddress) external view returns (address);

    function getAssetAccountAddressByNodeAddress(address nodeAddress) external view returns (address);

    function calculateNativeNodeRegistrationDigestHash(address assetAccountAddress, bytes32 salt, uint256 expiry)
        external
        view
        returns (bytes32);

    function domainSeparator() external view returns (bytes32);

    function assetAccountSaltIsSpent(address assetAccountAddress, bytes32 salt) external view returns (bool);
}
