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

    uint256 epoch = 0; // self.epoch, previously ined in adapter

    //  Node State Variables
    mapping(address => Node) public nodes; //maps node address to Node Struct
    mapping(address => uint256) public rewards; // maps node address to reward amount

    // Group State Variables
    uint256 public groupCount; // Number of groups
    mapping(uint256 => Group) public groups; // group_index => Group struct

    // Coordinators
    mapping(uint256 => address) public coordinators; // maps group index to coordinator address

    // * Structs
    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool state;
        uint256 pendingUntilBlock;
        uint256 staking;
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
        uint256 index;
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

    // ! Node Register
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

    function nodeJoin(address idAddress) private {
        // * get groupIndex from findOrCreateTargetGroup -> addGroup
        (uint256 groupIndex, bool needsRebalance) = findOrCreateTargetGroup();
        addToGroup(idAddress, groupIndex, true); // * add to group
        // TODO: Reblance Group: Implement later!
        // if (needsRebalance) {
        //     // reblanceGroup();
        // }
    }

    function reblanceGroup(uint256 groupIndexA, uint256 groupIndexB) private {
        Group storage groupA = groups[groupIndexA];
        Group storage groupB = groups[groupIndexB];

        // ? What is going on here.
    }

    function findOrCreateTargetGroup()
        private
        returns (
            uint256, //groupIndex
            bool // needsRebalance
        )
    {
        if (groupCount == 0) {
            uint256 groupIndex = addGroup();
            return (groupIndex, false);
        }
        return (1, false); // TODO: Need to implement index_of_min_size
    }

    function addGroup() internal returns (uint256) {
        groupCount++;
        Group storage g = groups[groupCount];
        g.index = groupCount;
        g.size = 0;
        g.threshold = DEFAULT_MINIMUM_THRESHOLD;
        return groupCount;
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
        m.index = g.size;
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

    function minimumThreshold(uint256 groupSize)
        internal
        pure
        returns (uint256)
    {
        // uint256 min = groupSize / 2 + 1;
        return groupSize / 2 + 1;
    }

    // ! Rust dkgtask struct
    // struct DKGTask {
    //     group_index: usize,
    //     epoch: usize,
    //     size: usize,
    //     threshold: usize,
    //     members: BTreeMap<String, usize>,
    //     assignment_block_height: usize,
    //     coordinator_address: String,
    // }

    event dkgTask(
        uint256 _groupIndex,
        uint256 _epoch,
        uint256 _size,
        uint256 _threshold,
        address[] _members,
        uint256 _assignmentBlockHeight,
        address _coordinatorAddress
    );

    function emitGroupEvent(uint256 groupIndex) internal {
        require(groups[groupIndex].index != 0, "Group does not exist");

        epoch++; // increment adapter epoch
        Group storage g = groups[groupIndex]; // Grab group struct
        g.epoch++; // Increment group epoch
        g.isStrictlyMajorityConsensusReached = false; // Reset consensus of group to false

        delete g.committers; // set commiters to empty
        delete g.commitCacheList; // Set commit_cache to empty
        // g.committers.push(address(5)); // ! Need to run experiments here.

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

        // TODO: Emit event
        emit dkgTask(
            g.index,
            g.epoch,
            g.size,
            g.threshold,
            groupNodes,
            block.number,
            address(coordinator)
        );
    }

    // ! Commit DKG
    function GetMemberIndex(uint256 groupIndex, address nodeIdAddress)
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

    /// Check to see if a group has a partial public key registered for a given node.
    function PartialKeyRegistered(uint256 groupIndex, address nodeIdAddress)
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
        // require group exists
        require(groups[params.groupIndex].index != 0, "Group does not exist");

        // require publickey and partial public key are not empty  / are the right format

        // require coordinator exists
        require(
            coordinators[params.groupIndex] != address(0),
            "Coordinator not found for groupIndex"
        );

        // Ensure DKG Proccess is in Phase
        ICoordinator coordinator = ICoordinator(
            coordinators[params.groupIndex]
        );
        // require(coordinator.inPhase() != -1, "DKG still in progress!"); // require coordinator to be in phase -1 (dkg end)
        require(coordinator.inPhase() != -1, "DKG has ended"); // require coordinator to still be in DKG Phase

        // Ensure Eopch is correct,  Node is in group, and has not already submitted a partial key
        Group storage g = groups[params.groupIndex]; // get group from group index
        require(
            params.groupEpoch == g.epoch,
            "Caller Group epoch does not match controller Group epoch"
        );

        require(
            GetMemberIndex(params.groupIndex, msg.sender) != -1, // -1 if node is not member of group
            "Node is not a member of the group"
        );

        // uint256 memberIndex = uint256(GetMemberIndex(groupIndex, msg.sender));

        require(
            !PartialKeyRegistered(params.groupIndex, msg.sender),
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
        if (g.isStrictlyMajorityConsensusReached) {
            g
            .members[uint256(GetMemberIndex(params.groupIndex, msg.sender))] // uint256 memberIndex
                .partialPublicKey = params.partialPublicKey;
        }

        // ! start
        // if not.. call getStrictlyMajorityIdenticalCommitmentResult for the group and check if consensus has been reached.
        if (!g.isStrictlyMajorityConsensusReached) {
            CommitCache
                memory identicalCommits = getStrictlyMajorityIdenticalCommitmentResult(
                    params.groupIndex
                );

            if (identicalCommits.nodeIdAddress.length != 0) {
                // TODO: let last_output = self.last_output as usize; // * What is this?
                // Get list of majority members with disqualified nodes excluded
                address[] memory majorityMembers = getMajorityMembers(
                    params.groupIndex,
                    identicalCommits.commitResult.disqualifiedNodes
                );
                if (majorityMembers.length >= g.threshold) {
                    g.isStrictlyMajorityConsensusReached = true;
                    g.size -= identicalCommits
                        .commitResult
                        .disqualifiedNodes
                        .length;
                    g.publicKey = identicalCommits.commitResult.publicKey;

                    //! Did my best here, but I think it's not quite there.
                    // Is majorityMembers the same at group.commitCache in the rust code?
                    // update partial public key of all non-disqualified members
                    g
                        .members[
                            uint256(
                                GetMemberIndex(params.groupIndex, msg.sender)
                            )
                        ]
                        .partialPublicKey = params.partialPublicKey;
                    for (uint256 i = 0; i < majorityMembers.length; i++) {
                        g
                            .members[
                                uint256(
                                    GetMemberIndex(
                                        params.groupIndex,
                                        majorityMembers[i]
                                    )
                                )
                            ]
                            .partialPublicKey = params.partialPublicKey;
                    }
                }
            }
        }
        // ! end

        // This works... the above fails.
        g
            .members[uint256(GetMemberIndex(params.groupIndex, msg.sender))]
            .partialPublicKey = params.partialPublicKey;

        if (!g.isStrictlyMajorityConsensusReached) {
            CommitCache
                memory identicalCommits = getStrictlyMajorityIdenticalCommitmentResult(
                    params.groupIndex
                );

            if (identicalCommits.nodeIdAddress.length != 0) {
                if (identicalCommits.nodeIdAddress.length >= g.threshold) {
                    g.isStrictlyMajorityConsensusReached = true;
                }
            }
        }
    }

    // function getMajorityMembers iterates through the members of a group and returns a list of members that are not in the list of disqualified nodes.
    function getMajorityMembers(
        uint256 groupIndex,
        address[] memory disqualifiedNodes
    ) public view returns (address[] memory) {
        Group storage g = groups[groupIndex];
        address[] memory majorityMembers = new address[](g.size);
        uint256 majorityMembersIndex = 0;
        for (uint256 i = 0; i < g.members.length; i++) {
            bool isDisqualified = false;
            for (uint256 j = 0; j < disqualifiedNodes.length; j++) {
                if (g.members[i].nodeIdAddress == disqualifiedNodes[j]) {
                    isDisqualified = true;
                }
            }
            if (!isDisqualified) {
                majorityMembers[majorityMembersIndex] = g
                    .members[i]
                    .nodeIdAddress;
                majorityMembersIndex++;
            }
        }
        return majorityMembers;
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

    // Goal: get array of majority members with identical commit result
    function getStrictlyMajorityIdenticalCommitmentResult(uint256 groupIndex)
        internal
        view
        returns (CommitCache memory)
    {
        CommitCache memory emptyCache = CommitCache(
            new address[](0),
            CommitResult(0, "", new address[](0))
        );

        Group memory g = groups[groupIndex];
        if (g.commitCacheList.length == 0) {
            return (emptyCache);
        }

        if (g.commitCacheList.length == 1) {
            return (g.commitCacheList[0]);
        }

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

        if (!isStrictlyMajorityExist) {
            return (emptyCache);
        }

        return (majorityCommitCache);
    }

    // ! Post Proccess DKG
    // Called by nodes after last phase of dkg ends (success or failure)
    // handles coordinator selfdestruct if it reaches DKG timeout, then
    // 1. emit GroupRelayTask if grouping successfully
    // 2. arrange members if fail to group
    // and rewards trigger (sender)
    function postProcessDkg(uint256 groupIndex, uint256 groupEpoch) public {
        // require group exists
        require(groups[groupIndex].index != 0, "Group does not exist");

        // require calling node is in group
        require(
            GetMemberIndex(groupIndex, msg.sender) != -1, // -1 if node is not member of group
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

        // Coordinator Self Destruct
        coordinator.selfDestruct();

        coordinators[groupIndex] = address(0);

        bool isStrictlyMajorityConsensusReached = g
            .isStrictlyMajorityConsensusReached;

        if (isStrictlyMajorityConsensusReached) {
            // TODO: Group relay task
        } else {
            // (
            //     bool consensusReached,
            //     address[] memory majority_members
            // ) = getStrictlyMajorityIdenticalCommitmentResult(groupIndex);
        }
    }

    // ************************************************** //
    // * Public Test functions for testing private stuff
    // * DELETE LATER
    // ************************************************** //

    function tNonexistantGroup(uint256 groupIndex) public {
        emitGroupEvent(groupIndex);
    }

    function tMinimumThreshold(uint256 groupSize)
        public
        pure
        returns (uint256)
    {
        return minimumThreshold(groupSize);
    }

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
