// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

interface IController {
    struct Group {
        uint256 index;
        uint256 epoch;
        uint256 size;
        uint256 threshold;
        Member[] members;
        address[] committers;
        CommitCache[] commitCacheList;
        bool isStrictlyMajorityConsensusReached;
        uint256[4] publicKey;
    }

    struct Member {
        address nodeIdAddress;
        uint256[4] partialPublicKey;
    }

    struct CommitResult {
        uint256 groupEpoch;
        uint256[4] publicKey;
        address[] disqualifiedNodes;
    }

    struct CommitCache {
        address[] nodeIdAddress;
        CommitResult commitResult;
    }

    struct CommitDkgParams {
        uint256 groupIndex;
        uint256 groupEpoch;
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
    }

    // node transaction
    function nodeJoin(address nodeIdAddress) external returns (uint256 groupIndex);

    function nodeLeave(address nodeIdAddress) external;

    function commitDkg(CommitDkgParams memory params) external;

    function postProcessDkg(uint256 groupIndex, uint256 groupEpoch) external;

    // nodeRegistry transaction
    function nodeWithdrawETH(address recipient, uint256 ethAmount) external;

    // adapter transaction
    function addReward(address[] memory nodes, uint256 ethAmount, uint256 arpaAmount) external;

    function setLastOutput(uint256 lastOutput) external;

    // view
    function getControllerConfig()
        external
        view
        returns (
            address nodeRegistryContractAddress,
            address adapterContractAddress,
            uint256 disqualifiedNodePenaltyAmount,
            uint256 defaultNumberOfCommitters,
            uint256 defaultDkgPhaseDuration,
            uint256 groupMaxCapacity,
            uint256 idealNumberOfGroups,
            uint256 dkgPostProcessReward
        );

    /// @notice Get list of all group indexes where group.isStrictlyMajorityConsensusReached == true
    /// @return uint256[] List of valid group indexes
    function getValidGroupIndices() external view returns (uint256[] memory);

    function getGroupEpoch() external view returns (uint256);

    function getGroupCount() external view returns (uint256);

    function getGroup(uint256 index) external view returns (Group memory);

    function getGroupThreshold(uint256 groupIndex) external view returns (uint256, uint256);

    function getMember(uint256 groupIndex, uint256 memberIndex) external view returns (Member memory);

    /// @notice Get the group index and member index of a given node.
    function getBelongingGroup(address nodeAddress) external view returns (int256, int256);

    function getCoordinator(uint256 groupIndex) external view returns (address);

    function getLastOutput() external view returns (uint256);

    /// @notice Check to see if a group has a partial public key registered for a given node.
    /// @return bool True if the node has a partial public key registered for the group.
    function isPartialKeyRegistered(uint256 groupIndex, address nodeIdAddress) external view returns (bool);
}
