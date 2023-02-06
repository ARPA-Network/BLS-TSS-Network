pragma solidity ^0.8.15;

import "openzeppelin-contracts/contracts/access/Ownable.sol";
import "openzeppelin-contracts/contracts/utils/math/SafeMath.sol";

import {Coordinator} from "src/Coordinator.sol";
import "src/ICoordinator.sol";

contract Controller is Ownable {
    uint256 public constant NODE_STAKING_AMOUNT = 50000;
    uint256 public constant DISQUALIFIED_NODE_PENALTY_AMOUNT = 1000;
    uint256 public constant COORDINATOR_STATE_TRIGGER_REWARD = 100;
    uint256 public constant DEFAULT_MINIMUM_THRESHOLD = 3;
    uint256 public constant DEFAULT_NUMBER_OF_COMMITTERS = 3;
    uint256 public constant DEFAULT_DKG_PHASE_DURATION = 10;
    uint256 public constant GROUP_MAX_CAPACITY = 10;
    uint256 public constant IDEAL_NUMBER_OF_GROUPS = 5;
    uint256 public constant PENDING_BLOCK_AFTER_QUIT = 100;
    uint256 public constant DKG_POST_PROCESS_REWARD = 100;

    uint256 epoch = 0; // self.epoch, previously ined in adapter

    //  Node State Variables
    mapping(address => Node) public nodes; //maps node address to Node Struct
    mapping(address => uint256) public rewards; // maps node address to reward amount

    // Group State Variables
    uint256 public groupCount; // Number of groups
    mapping(uint256 => Group) public groups; // group_index => Group struct
    mapping(uint256 => address) public coordinators; // maps group index to coordinator address
    uint64 lastOutput = 0x2222222222222222; // global last output

    // * Structs
    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool state;
        uint256 pendingUntilBlock;
        uint256 staking; // seems to be a duplicate here to the rewards mapping... which should I use?
    }
    struct Group {
        uint256 index; // group_index
        uint256 epoch; // 0
        uint256 size; // 0
        uint256 threshold; // DEFAULT_MINIMUM_THRESHOLD
        Member[] members; // Map in rust mock contract
        address[] committers;
        CommitCache[] commitCacheList; // Map in rust mock contract
        bool isStrictlyMajorityConsensusReached;
        bytes publicKey;
    }

    struct Member {
        address nodeIdAddress;
        bytes partialPublicKey;
    }

    struct CommitResult {
        uint256 groupEpoch;
        bytes publicKey;
        address[] disqualifiedNodes;
    }

    struct CommitCache {
        address[] nodeIdAddress;
        CommitResult commitResult;
    }

    function nodeRegister(bytes calldata dkgPublicKey) public {
        require(
            nodes[msg.sender].idAddress == address(0),
            "Node is already registered"
        ); // error sender already in list of nodes

        // TODO: Check to see if enough balance for staking

        // Populate Node struct and insert into nodes
        Node storage n = nodes[msg.sender];
        n.idAddress = msg.sender;
        n.dkgPublicKey = dkgPublicKey;
        n.state = true;
        n.pendingUntilBlock = 0;
        n.staking = NODE_STAKING_AMOUNT;

        nodeJoin(msg.sender);
    }

    function nodeJoin(address idAddress) internal {
        // get groupIndex from findOrCreateTargetGroup -> addGroup
        (uint256 groupIndex, bool needsRebalance) = findOrCreateTargetGroup();

        addToGroup(idAddress, groupIndex, true); // add node to group

        // If needs rebalance,
        if (needsRebalance) {
            // Get list of all group indicies excluding the current group index.
            uint256[] memory groupIndices = new uint256[](groupCount - 1);
            uint256 index = 0;
            for (uint256 i = 0; i < groupCount; i++) {
                if (groupIndex != i) {
                    groupIndices[index] = i;
                    index++;
                }
            }

            // iterate over group indices and attempt to rebalance group, break as soon as success
            // Rebalance group. Group A Index = iterate over each group other than Group B Index.
            for (uint256 i = 0; i < groupIndices.length; i++) {
                if (rebalanceGroup(groupIndices[i], groupIndex)) {
                    break;
                }
            }
        }
    }

    function rebalanceGroup(
        uint256 groupAIndex,
        uint256 groupBIndex // Needs further testing
    ) public returns (bool) {
        Group memory groupA = groups[groupAIndex];
        Group memory groupB = groups[groupBIndex];

        if (groupB.size > groupA.size) {
            (groupA, groupB) = (groupB, groupA); // Swap groupA and groupB
            (groupAIndex, groupBIndex) = (groupBIndex, groupAIndex); // Swap groupAIndex and groupBIndex
        }

        uint256 expectedSizeToMove = groupA.size -
            (groupA.size + groupB.size) /
            2;
        if (
            expectedSizeToMove == 0 ||
            groupA.size - expectedSizeToMove < DEFAULT_MINIMUM_THRESHOLD
        ) {
            return false;
        }

        uint256[] memory qualifiedIndices = new uint256[](
            groupA.members.length
        );
        for (uint256 i = 0; i < groupA.members.length; i++) {
            qualifiedIndices[i] = i;
        }

        uint256[] memory membersToMove = chooseRandomlyFromIndices(
            lastOutput,
            qualifiedIndices,
            expectedSizeToMove
        );

        // Move members from group A to group B
        for (uint256 i = 0; i < membersToMove.length; i++) {
            uint256 memberIndex = membersToMove[i];
            address idAddress = getMemberAddressByIndex(
                groupAIndex,
                memberIndex
            );
            removeFromGroup(idAddress, groupAIndex, false);
            addToGroup(idAddress, groupBIndex, false);
        }

        emitGroupEvent(groupAIndex);
        emitGroupEvent(groupBIndex);

        return true;
    }

    function removeFromGroup(
        address nodeIdAddress,
        uint256 groupIndex,
        bool emitEventInstantly
    ) public returns (bool) {
        Group storage group = groups[groupIndex];

        group.size--;

        if (group.size == 0) {
            return false;
        }

        // code to pop and resize array instead of delete
        uint256 foundIndex;
        for (uint256 i = 0; i < group.members.length; i++) {
            if (group.members[i].nodeIdAddress == nodeIdAddress) {
                foundIndex = i;
                break;
            }
        }
        group.members[foundIndex] = group.members[group.members.length - 1];
        group.members.pop();

        uint256 minimum = minimumThreshold(group.size);

        group.threshold = minimum > DEFAULT_MINIMUM_THRESHOLD
            ? minimum
            : DEFAULT_MINIMUM_THRESHOLD;

        if (group.size < 3) {
            return true;
        }

        if (emitEventInstantly) {
            emitGroupEvent(groupIndex);
        }

        return false;
    }

    function findOrCreateTargetGroup()
        public
        returns (
            uint256, //groupIndex
            bool // needsRebalance
        )
    {
        if (groupCount == 0) {
            // if group is empty, addgroup.
            uint256 groupIndex = addGroup();
            return (groupIndex, false);
        }

        // get the group index of the group with the minimum size, as well as the min size
        uint256 indexOfMinSize;
        uint256 minSize = GROUP_MAX_CAPACITY;
        for (uint256 i = 0; i < groupCount; i++) {
            Group memory g = groups[i];
            if (g.size < minSize) {
                minSize = g.size;
                indexOfMinSize = i;
            }
        }

        // compute the valid group count
        uint256 validGroupCount = validGroupIndices().length;

        // check if valid group count < ideal_number_of_groups || minSize == group_max_capacity
        // If either condition is met and the number of valid groups == group count, call add group and return (index of new group, true)
        if (
            (validGroupCount < IDEAL_NUMBER_OF_GROUPS &&
                validGroupCount == groupCount) ||
            (minSize == GROUP_MAX_CAPACITY)
        ) {
            uint256 groupIndex = addGroup();
            return (groupIndex, true); // NEEDS REBALANCE
        }

        // if none of the above conditions are met:
        return (indexOfMinSize, false);
    }

    // Get list of all group indexes where group.isStrictlyMajorityConsensusReached == true
    function validGroupIndices() public view returns (uint256[] memory) {
        uint256[] memory groupIndices = new uint256[](groupCount); //max length is group count
        uint256 index = 0;
        for (uint256 i = 0; i < groupCount; i++) {
            Group memory g = groups[i];
            if (g.isStrictlyMajorityConsensusReached) {
                groupIndices[index] = i;
                index++;
            }
        }

        // create result array of correct size (remove possible trailing zero elements)
        uint256[] memory result = new uint256[](index);
        for (uint256 i = 0; i < index; i++) {
            result[i] = groupIndices[i];
        }

        return result;
    }

    function addGroup() internal returns (uint256) {
        uint256 groupIndex = groupCount; // groupIndex starts at 0. groupCount is index of next group to be added
        groupCount++;

        Group storage g = groups[groupIndex];
        g.index = groupIndex;
        g.size = 0;
        g.threshold = DEFAULT_MINIMUM_THRESHOLD;

        return groupIndex;
    }

    function addToGroup(
        address idAddress,
        uint256 groupIndex,
        bool emitEventInstantly
    ) internal {
        // Get group from group index
        Group storage g = groups[groupIndex];

        // Add Member Struct to group at group index
        Member memory m;
        m.nodeIdAddress = idAddress;

        // insert (node id address - > member) into group.members
        g.members.push(m);
        g.size++;

        // assign group threshold
        uint256 minimum = minimumThreshold(g.size); // 51% of group size
        // max of 51% of group size and DEFAULT_MINIMUM_THRESHOLD
        g.threshold = minimum > DEFAULT_MINIMUM_THRESHOLD
            ? minimum
            : DEFAULT_MINIMUM_THRESHOLD;

        if ((g.size >= 3) && emitEventInstantly) {
            emitGroupEvent(groupIndex);
        }
    }

    // returns the minimum threshold for a group of size groupSize
    function minimumThreshold(
        uint256 groupSize // set this to internal later
    ) public pure returns (uint256) {
        return groupSize / 2 + 1;
    }

    event dkgTask(
        uint256 _groupIndex,
        uint256 _epoch,
        uint256 _size,
        uint256 _threshold,
        address[] _members,
        uint256 _assignmentBlockHeight,
        address _coordinatorAddress
    );

    function emitGroupEvent(uint256 groupIndex) public {
        // Set to internal later
        // require(groups[groupIndex].index < groupCount, "Group does not exist");
        require(groupIndex < groupCount, "Group does not exist");

        epoch++; // increment adapter epoch
        Group storage g = groups[groupIndex]; // Grab group struct
        g.epoch++; // Increment group epoch
        g.isStrictlyMajorityConsensusReached = false; // Reset consensus of group to false

        delete g.committers; // set commiters to empty
        delete g.commitCacheList; // Set commit_cache to empty

        // Deploy coordinator, add to coordinators mapping
        Coordinator coordinator;
        coordinator = new Coordinator(g.threshold, DEFAULT_DKG_PHASE_DURATION);
        coordinators[groupIndex] = address(coordinator);

        // Initialize Coordinator
        address[] memory groupNodes = new address[](g.size);
        bytes[] memory groupKeys = new bytes[](g.size);

        for (uint256 i = 0; i < g.size; i++) {
            groupNodes[i] = g.members[i].nodeIdAddress;
            groupKeys[i] = nodes[g.members[i].nodeIdAddress].dkgPublicKey;
        }

        coordinator.initialize(groupNodes, groupKeys);

        emit dkgTask( // needs to be verified against what node is expecting
            g.index,
            g.epoch,
            g.size,
            g.threshold,
            groupNodes,
            block.number,
            address(coordinator)
        );
    }

    function getMemberIndexByAddress(uint256 groupIndex, address nodeIdAddress)
        public
        view
        returns (int256 memberIndex)
    {
        Group storage g = groups[groupIndex];
        for (uint256 i = 0; i < g.members.length; i++) {
            if (g.members[i].nodeIdAddress == nodeIdAddress) {
                return int256(i);
            }
        }
        return -1;
    }

    function getMemberAddressByIndex(uint256 groupIndex, uint256 memberIndex)
        public
        view
        returns (address nodeIdAddress)
    {
        Group storage g = groups[groupIndex];
        return g.members[memberIndex].nodeIdAddress;
    }

    /// Check to see if a group has a partial public key registered for a given node.
    function partialKeyRegistered(uint256 groupIndex, address nodeIdAddress)
        public
        view
        returns (bool)
    {
        Group storage g = groups[groupIndex];
        for (uint256 i = 0; i < g.members.length; i++) {
            if (
                g.members[i].nodeIdAddress == nodeIdAddress &&
                g.members[i].partialPublicKey.length != 0
            ) {
                return true;
            }
        }
        return false;
    }

    struct CommitDkgParams {
        uint256 groupIndex;
        uint256 groupEpoch;
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
    }

    function commitDkg(CommitDkgParams memory params) external {
        require(params.groupIndex < groupCount, "Group does not exist");

        // Todo: require publickey and partial public key are not empty  / are the right format

        // require coordinator exists
        require(
            coordinators[params.groupIndex] != address(0),
            "Coordinator not found for groupIndex"
        );

        // Ensure DKG Proccess is in Phase
        ICoordinator coordinator = ICoordinator(
            coordinators[params.groupIndex]
        );
        require(coordinator.inPhase() != -1, "DKG has ended"); // require coordinator to still be in DKG Phase

        // Ensure Eopch is correct,  Node is in group, and has not already submitted a partial key
        Group storage g = groups[params.groupIndex]; // get group from group index
        require(
            params.groupEpoch == g.epoch,
            "Caller Group epoch does not match controller Group epoch"
        );

        require(
            getMemberIndexByAddress(params.groupIndex, msg.sender) != -1, // -1 if node is not member of group
            "Node is not a member of the group"
        );

        require( // check to see if member has called commitdkg in the past.
            !partialKeyRegistered(params.groupIndex, msg.sender),
            "CommitCache already contains PartialKey for this node"
        );

        // Populate CommitResult / CommitCache
        CommitResult memory commitResult = CommitResult({
            groupEpoch: params.groupEpoch,
            publicKey: params.publicKey,
            disqualifiedNodes: params.disqualifiedNodes
        });

        if (!tryAddToExistingCommitCache(params.groupIndex, commitResult)) {
            CommitCache memory commitCache = CommitCache({
                commitResult: commitResult,
                nodeIdAddress: new address[](1)
            });

            commitCache.nodeIdAddress[0] = msg.sender;
            g.commitCacheList.push(commitCache);
        }

        // if consensus previously reached, update the partial public key of the given node's member entry in the group
        g
            .members[
                uint256(getMemberIndexByAddress(params.groupIndex, msg.sender))
            ]
            .partialPublicKey = params.partialPublicKey;

        // if not.. call get StrictlyMajorityIdenticalCommitmentResult for the group and check if consensus has been reached.
        if (!g.isStrictlyMajorityConsensusReached) {
            CommitCache
                memory identicalCommits = getStrictlyMajorityIdenticalCommitmentResult(
                    params.groupIndex
                );

            if (identicalCommits.nodeIdAddress.length != 0) {
                // TODO: let last_output = self.last_output as usize; // * What is this?

                address[] memory disqualifiedNodes = identicalCommits
                    .commitResult
                    .disqualifiedNodes;

                // Get list of majority members with disqualified nodes excluded
                address[]
                    memory majorityMembers = getNonDisqualifiedMajorityMembers(
                        identicalCommits.nodeIdAddress,
                        disqualifiedNodes
                    );

                if (majorityMembers.length >= g.threshold) {
                    // Remove all members from group where member.nodeIdAddress is in the disqualified nodes.
                    for (uint256 i = 0; i < disqualifiedNodes.length; i++) {
                        for (uint256 j = 0; j < g.members.length; j++) {
                            if (
                                g.members[j].nodeIdAddress ==
                                disqualifiedNodes[i]
                            ) {
                                g.members[j] = g.members[g.members.length - 1];
                                g.members.pop();
                                break;
                            }
                        }
                    }

                    // Update group with new values
                    g.isStrictlyMajorityConsensusReached = true;
                    g.size -= identicalCommits
                        .commitResult
                        .disqualifiedNodes
                        .length;
                    g.publicKey = identicalCommits.commitResult.publicKey;

                    // Create indexMemberMap: Iterate through group.members and create mapping: memberIndex -> nodeIdAddress
                    // Create qualifiedIndices: Iterate through group, add all member indexes found in majorityMembers.
                    uint256[] memory qualifiedIndices = new uint256[](
                        majorityMembers.length
                    );

                    for (uint256 j = 0; j < majorityMembers.length; j++) {
                        for (uint256 i = 0; i < g.members.length; i++) {
                            if (
                                g.members[i].nodeIdAddress == majorityMembers[j]
                            ) {
                                qualifiedIndices[j] = i;
                                break;
                            }
                        }
                    }

                    // Compute commiter_indices by calling chooseRandomlyFromIndices with qualifiedIndices as input.
                    uint256[]
                        memory committerIndices = chooseRandomlyFromIndices(
                            lastOutput,
                            qualifiedIndices,
                            DEFAULT_NUMBER_OF_COMMITTERS
                        );

                    // For selected commiter_indices: add corresponding members into g.committers
                    g.committers = new address[](committerIndices.length);
                    for (uint256 i = 0; i < committerIndices.length; i++) {
                        g.committers[i] = g
                            .members[committerIndices[i]]
                            .nodeIdAddress;
                    }

                    // Iterate over disqualified nodes and call slashNode on each.
                    for (uint256 i = 0; i < disqualifiedNodes.length; i++) {
                        slashNode(
                            disqualifiedNodes[i],
                            DISQUALIFIED_NODE_PENALTY_AMOUNT,
                            0,
                            false
                        );
                    }
                }
            }
        }
    } // end commitDkg

    // temporarily public for testing. This should be internal.

    // Choose "count" random indices from "indices" array.
    function chooseRandomlyFromIndices(
        uint64 seed,
        uint256[] memory indices,
        uint256 count
    ) public pure returns (uint256[] memory) {
        uint256[] memory chosenIndices = new uint256[](count);

        // Create copy of indices to avoid modifying original array.
        uint256[] memory remainingIndices = new uint256[](indices.length);
        for (uint256 i = 0; i < indices.length; i++) {
            remainingIndices[i] = indices[i];
        }

        uint256 remainingCount = remainingIndices.length;
        for (uint256 i = 0; i < count; i++) {
            uint256 index = uint256(keccak256(abi.encodePacked(seed, i))) %
                remainingCount;
            chosenIndices[i] = remainingIndices[index];
            remainingIndices[index] = remainingIndices[remainingCount - 1];
            remainingCount--;
        }
        return chosenIndices;
    }

    // Goal: get array of majority members with identical commit result. Return commit cache. if no majority, return empty commit cache.
    function getStrictlyMajorityIdenticalCommitmentResult(uint256 groupIndex)
        internal
        view
        returns (CommitCache memory)
    {
        CommitCache memory emptyCache = CommitCache(
            new address[](0),
            CommitResult(0, "", new address[](0))
        );

        // If there are no commit caches, return empty commit cache.
        Group memory g = groups[groupIndex];
        if (g.commitCacheList.length == 0) {
            return (emptyCache);
        }

        // If there is only one commit cache, return it.
        if (g.commitCacheList.length == 1) {
            return (g.commitCacheList[0]);
        }

        // If there are multiple commit caches, check if there is a majority.  (THIS NEEDS INVESTIGAGION...)
        bool isStrictlyMajorityExist = true;
        CommitCache memory majorityCommitCache = g.commitCacheList[0];
        for (uint256 i = 1; i < g.commitCacheList.length; i++) {
            CommitCache memory commitCache = g.commitCacheList[i];
            if (
                commitCache.nodeIdAddress.length >
                majorityCommitCache.nodeIdAddress.length
            ) {
                isStrictlyMajorityExist = true;
                majorityCommitCache = commitCache;
            } else if (
                commitCache.nodeIdAddress.length ==
                majorityCommitCache.nodeIdAddress.length
            ) {
                isStrictlyMajorityExist = false;
            }
        }

        // If no majority, return empty commit cache.
        if (!isStrictlyMajorityExist) {
            return (emptyCache);
        }
        // If majority, return majority commit cache
        return (majorityCommitCache);
    }

    // function getNonDisqualifiedMajorityMembers iterates through list of members and remove disqualified nodes.
    function getNonDisqualifiedMajorityMembers(
        address[] memory nodeAddresses,
        address[] memory disqualifiedNodes
    ) public pure returns (address[] memory) {
        address[] memory majorityMembers = new address[](nodeAddresses.length);
        uint256 majorityMembersLength = 0;
        for (uint256 i = 0; i < nodeAddresses.length; i++) {
            bool isDisqualified = false;
            for (uint256 j = 0; j < disqualifiedNodes.length; j++) {
                if (nodeAddresses[i] == disqualifiedNodes[j]) {
                    isDisqualified = true;
                    break;
                }
            }
            if (!isDisqualified) {
                majorityMembers[majorityMembersLength] = nodeAddresses[i];
                majorityMembersLength++;
            }
        }

        // remove trailing zero addresses
        address[] memory output = new address[](majorityMembersLength);
        for (uint256 i = 0; i < majorityMembersLength; i++) {
            output[i] = majorityMembers[i];
        }

        return output;
    }

    function tryAddToExistingCommitCache(
        uint256 groupIndex,
        CommitResult memory commitResult
    ) internal returns (bool isExist) {
        Group storage g = groups[groupIndex]; // get group from group index
        for (uint256 i = 0; i < g.commitCacheList.length; i++) {
            if (
                keccak256(abi.encode(g.commitCacheList[i].commitResult)) ==
                keccak256(abi.encode(commitResult))
            ) {
                // isExist = true;
                g.commitCacheList[i].nodeIdAddress.push(msg.sender);
                return true;
            }
        }
    }

    event groupRelayTask(
        uint256 index,
        uint256 relayedGroupIndex,
        uint256 relayedGroupEpoch,
        uint256 assignmentBlockHeight
    );

    function postProcessDkg(uint256 groupIndex, uint256 groupEpoch) public {
        // require group exists
        // require(groups[groupIndex].index != 0, "Group does not exist");
        require(groupIndex < groupCount, "Group does not exist"); // Is this okay?

        // require calling node is in group
        require(
            getMemberIndexByAddress(groupIndex, msg.sender) != -1, // -1 if node is not member of group
            "Node is not a member of the group"
        );
        // require correct epoch
        Group storage g = groups[groupIndex];
        require(
            groupEpoch == g.epoch,
            "Caller Group epoch does not match Controller Group epoch"
        );

        // require coordinator exists
        require(
            coordinators[groupIndex] != address(0),
            "Coordinator not found for groupIndex"
        );

        // Require DKG Proccess is in Phase
        ICoordinator coordinator = ICoordinator(coordinators[groupIndex]);
        require(coordinator.inPhase() == -1, "DKG still in progress"); // require DKG Phase End.

        // delete coordinator
        coordinator.selfDestruct(); // coordinator self destruct // ! might be deprecated
        coordinators[groupIndex] = address(0); // remove coordinator from mapping

        // check if majority consensus reached
        bool isStrictlyMajorityConsensusReached = g
            .isStrictlyMajorityConsensusReached;

        // get strictly majority identical commitment result
        CommitCache
            memory majorityMembers = getStrictlyMajorityIdenticalCommitmentResult(
                groupIndex
            );

        if (!isStrictlyMajorityConsensusReached) {
            if (groupCount > 1) {
                emit groupRelayTask( //! does this look okay?
                    epoch,
                    groupIndex,
                    groupEpoch,
                    block.number
                );
            }
        } else {
            if (majorityMembers.nodeIdAddress.length == 0) {
                // if empty cache: zero out group
                g.size = 0;
                g.threshold = 0;

                // for each member, slash node
                for (uint256 i = 0; i < g.members.length; i++) {
                    slashNode(
                        g.members[i].nodeIdAddress,
                        DISQUALIFIED_NODE_PENALTY_AMOUNT,
                        0, //! should this be 0?
                        false
                    );
                }

                delete g.members; // Delete all members of the group
            } else {
                // get disqualified nodes
                address[] memory disqualifiedNodes = majorityMembers
                    .commitResult
                    .disqualifiedNodes;
                g.size -= disqualifiedNodes.length;
                uint256 minimum = minimumThreshold(g.size);

                // set g.threshold to max (default min threshold / minimum threshold)
                g.threshold = g.threshold > minimum
                    ? DEFAULT_MINIMUM_THRESHOLD
                    : minimum;

                // Delete disqualified members from group
                for (uint256 i = 0; i < g.members.length; i++) {
                    for (uint256 j = 0; j < disqualifiedNodes.length; j++) {
                        if (
                            g.members[i].nodeIdAddress == disqualifiedNodes[j]
                        ) {
                            delete g.members[i];
                            break;
                        }
                    }
                }

                // for each disqualified node, slash node
                for (uint256 i = 0; i < disqualifiedNodes.length; i++) {
                    bool handleGroup = (i == disqualifiedNodes.length - 1);
                    slashNode(
                        disqualifiedNodes[i],
                        DISQUALIFIED_NODE_PENALTY_AMOUNT,
                        0, //! should this be 0?
                        handleGroup
                    );
                }
            }
        }

        // update rewards for calling node
        rewards[msg.sender] += DKG_POST_PROCESS_REWARD;
    }

    function getRewards(address nodeAddress) public view returns (uint256) {
        return rewards[nodeAddress];
    }

    function getStakedAmount(address nodeAddress)
        public
        view
        returns (uint256)
    {
        Node storage node = nodes[nodeAddress];
        require(node.idAddress == nodeAddress, "Node not registered.");
        return node.staking;
    }

    function nodeStake(uint256 stakeAmount) public {
        Node storage node = nodes[msg.sender];
        require(node.idAddress == msg.sender, "Node not registered.");
        node.staking += stakeAmount;
    }

    function nodeUnstake(uint256 unstakeAmount) public {
        Node storage node = nodes[msg.sender];
        require(node.idAddress == msg.sender, "Node not registered.");

        if (node.state == true) {
            require(
                node.staking - unstakeAmount >= NODE_STAKING_AMOUNT,
                "Node state is true, cannot unstake below staking threshold"
            );
        }

        // require(node.staking - unstakeAmount >= 0, "Cannot unstake more than amount staked"); //need this?

        node.staking -= unstakeAmount;
    }

    function slashNode(
        address nodeIdAddress,
        uint256 stakingPenalty,
        uint256 pendingBlock,
        bool handleGroup // flip to internal
    ) public {
        Node storage node = nodes[nodeIdAddress];
        node.staking -= stakingPenalty;
        if (node.staking < NODE_STAKING_AMOUNT || pendingBlock > 0) {
            freezeNode(nodeIdAddress, pendingBlock, handleGroup);
        }
    }

    // removes node from the group
    function freezeNode(
        address nodeIdAddress,
        uint256 pendingBlock,
        bool handleGroup
    ) public {
        // flip to internal
        if (handleGroup) {
            uint256 groupIndex;
            bool groupFound = false;
            // find group with member = nodeIdAddress
            for (uint256 i = 0; i < groupCount; i++) {
                if (getMemberIndexByAddress(i, nodeIdAddress) != -1) {
                    groupIndex = i;
                    groupFound = true;
                    break;
                }
            }

            if (groupFound) {
                bool needsRebalance = removeFromGroup(
                    nodeIdAddress,
                    groupIndex,
                    true
                );
                // TODO check if the group ready to dkg? (This was not yet implemented in commit dkg.)
                if (needsRebalance) {
                    // set isStrictlyMajorityConsensusReached to false
                    groups[groupIndex]
                        .isStrictlyMajorityConsensusReached = false;

                    // get all group indices excluding the current groupIndex
                    uint256[] memory groupIndices = new uint256[](
                        groupCount - 1
                    );
                    uint256 index = 0;
                    for (uint256 i = 0; i < groupCount; i++) {
                        if (i != groupIndex) {
                            groupIndices[index] = i;
                            index++;
                        }
                    }

                    // try to reblance each group, if a any fail, set rebalanceFailure to true
                    bool rebalanceFailure = false;
                    for (uint256 i = 0; i < groupIndices.length; i++) {
                        if (!rebalanceGroup(i, groupIndex)) {
                            rebalanceFailure = true;
                        }
                    }
                    // if rebalance failed, start DKG for groupIndex
                    if (rebalanceFailure) {
                        // collect idAddress of members in group
                        address[] memory membersLeftInGroup;
                        for (uint256 i = 0; i < groups[groupIndex].size; i++) {
                            membersLeftInGroup[i] = groups[groupIndex]
                                .members[i]
                                .nodeIdAddress;
                        }

                        // for each membersLeftInGroup, call findOrCreateTargetGroup and then add that member to the new group.
                        for (
                            uint256 i = 0;
                            i < membersLeftInGroup.length;
                            i++
                        ) {
                            // find a suitable group for the member
                            (
                                uint256 targetGroupIndex,
                                bool _needsRebalance
                            ) = findOrCreateTargetGroup();

                            delete (_needsRebalance); //! Used to supress "unused parameter" warning, should I just leave this out?

                            // if the current group index is selected, break
                            if (groupIndex == targetGroupIndex) {
                                break;
                            }

                            // add member to target group
                            addToGroup(
                                membersLeftInGroup[i],
                                targetGroupIndex,
                                false
                            );

                            // if group at targetGroupIndex now has 3+ members, emit group event
                            if (
                                groups[targetGroupIndex].size >=
                                DEFAULT_MINIMUM_THRESHOLD
                            ) {
                                emitGroupEvent(targetGroupIndex);
                            }
                        }
                    }
                }
            }
        }
        // set node state to false for frozen node
        nodes[nodeIdAddress].state = false;

        uint256 currentBlock = block.number;
        // if the node is already pending, add the pending block to the current pending block
        if (nodes[nodeIdAddress].pendingUntilBlock > block.number) {
            nodes[nodeIdAddress].pendingUntilBlock += pendingBlock;
            // else set the pending block to the current block + pending block
        } else {
            nodes[nodeIdAddress].pendingUntilBlock =
                currentBlock +
                pendingBlock;
        }
    }

    // ************************************************** //
    // * Public Test functions for testing private stuff
    // * DELETE LATER
    // ************************************************** //

    function getNode(address nodeAddress) public view returns (Node memory) {
        return nodes[nodeAddress];
    }

    function getGroup(uint256 groupIndex) public view returns (Group memory) {
        return groups[groupIndex];
    }

    function getMember(uint256 groupIndex, uint256 memberIndex)
        public
        view
        returns (Member memory)
    {
        return groups[groupIndex].members[memberIndex];
    }

    function getCoordinator(uint256 groupIndex) public view returns (address) {
        return coordinators[groupIndex];
    }
}
