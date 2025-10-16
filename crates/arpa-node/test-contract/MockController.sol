// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MockController {
    mapping(uint256 => Group) private groups;
    address public nodeRegistryAddress;
    address public adapterAddress;

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

    event DkgTask(
        uint256 indexed globalEpoch,
        uint256 indexed groupIndex,
        uint256 indexed groupEpoch,
        uint256 size,
        uint256 threshold,
        address[] members,
        uint256 assignmentBlockHeight,
        address coordinatorAddress
    );

    constructor(address _nodeRegistryAddress) {
        nodeRegistryAddress = _nodeRegistryAddress;
        adapterAddress = address(0);
    }

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
        )
    {
        return (
            nodeRegistryAddress,
            adapterAddress,
            1000, // disqualifiedNodePenaltyAmount
            5, // defaultNumberOfCommitters
            100, // defaultDkgPhaseDuration
            10, // groupMaxCapacity
            3, // idealNumberOfGroups
            100 // dkgPostProcessReward
        );
    }

    function setGroup(
        uint256 groupIndex,
        uint256 epoch,
        uint256 size,
        uint256 threshold,
        bool isStrictlyMajorityConsensusReached,
        uint256[4] memory publicKey,
        address[] memory memberAddresses
    ) external {
        Group storage group = groups[groupIndex];
        group.index = groupIndex;
        group.epoch = epoch;
        group.size = size;
        group.threshold = threshold;
        group.isStrictlyMajorityConsensusReached = isStrictlyMajorityConsensusReached;
        group.publicKey = publicKey;

        delete group.members;
        for (uint256 i = 0; i < memberAddresses.length; i++) {
            uint256[4] memory emptyPartialPublicKey;
            group.members.push(Member({nodeIdAddress: memberAddresses[i], partialPublicKey: emptyPartialPublicKey}));
        }

        group.committers = memberAddresses;
    }

    function setMemberPartialPublicKey(uint256 groupIndex, uint256 memberIndex, uint256[4] memory partialPublicKey)
        external
    {
        groups[groupIndex].members[memberIndex].partialPublicKey = partialPublicKey;
    }

    function setCommitters(uint256 groupIndex, address[] memory committerAddresses) external {
        groups[groupIndex].committers = committerAddresses;
    }

    function getGroup(uint256 groupIndex) public view returns (Group memory) {
        return groups[groupIndex];
    }

    function emitDkgTaskEvent(
        uint256 globalEpoch,
        uint256 groupIndex,
        uint256 groupEpoch,
        uint256 size,
        uint256 threshold,
        address[] memory members,
        uint256 assignmentBlockHeight,
        address coordinatorAddress
    ) external {
        emit DkgTask(
            globalEpoch, groupIndex, groupEpoch, size, threshold, members, assignmentBlockHeight, coordinatorAddress
        );
    }

    function setAdapterAddress(address _adapterAddress) external {
        adapterAddress = _adapterAddress;
    }
}
