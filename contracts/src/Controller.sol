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
    mapping(address => bool) public nodeRegistered; // map for checking if nodes are registered

    // Group State Variables
    uint256 public groupCount; // Number of groups
    mapping(uint256 => Group) public groups; // group_index => Group struct
    mapping(uint256 => bool) public groupRegistered; // map for checking if group exists

    // Coordinators
    mapping(uint256 => address) public coordinators; // maps group index to coordinator address

    // * Structs
    struct Node {
        address idAddress;
        bytes dkgPublicKey;
        bool state;
        uint256 pending_until_block;
        uint256 staking;
    }
    struct Group {
        uint256 index; // group_index
        uint256 epoch; // 0
        uint256 size; // 0
        uint256 threshold; // DEFAULT_MINIMUM_THRESHOLD
        Member[] members; // Map in rust mock contract
        address[] committers;
        CommitCache[] commitCache; // Map in rust mock contract
        bool isStrictlyMajorityConsensusReached;
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
        address nodeIdAddress;
        CommitResult commitResult;
        bytes partialPublicKey;
    }

    // * Functions
    function nodeRegister(bytes calldata dkgPublicKey) public {
        require(!nodeRegistered[msg.sender], "Node is already registered"); // error sender already in list of nodes

        // TODO: Check to see if enough balance for staking

        // Populate Node struct and insert into nodes
        Node storage n = nodes[msg.sender];
        n.idAddress = msg.sender;
        n.dkgPublicKey = dkgPublicKey;
        n.state = true;
        n.pending_until_block = 0;
        n.staking = NODE_STAKING_AMOUNT;

        nodeRegistered[msg.sender] = true;
        rewards[msg.sender] = 0; // This can be removed
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

    // function reblanceGroup(uint256 groupIndexA, uint256 groupIndexB) private {}

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
        groupRegistered[groupCount] = true;
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
        uint256 min = groupSize / 2 + 1;
        return min;
    }

    function emitGroupEvent(uint256 groupIndex) internal {
        require(groupRegistered[groupIndex], "Group does not exist"); // group must exist

        epoch++; // increment adapter epoch
        Group storage g = groups[groupIndex]; // Grap group struct
        g.epoch++; // Increment group epoch

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
        // dkgtask = {}
        // emit_dkg_task (dkg_task) -> this let nodes know to start DKG with the coordinator
    }

    // ! Commit DKG
    // groupindex -> member registered -> true / false
    // ! Drop storage mappings and iterate via view function

    function NodeInMembers(uint256 groupIndex, address nodeIdAddress)
        public
        view
        returns (bool)
    {
        Group storage g = groups[groupIndex];
        for (uint256 i = 0; i < g.members.length; i++) {
            if (g.members[i].nodeIdAddress == nodeIdAddress) {
                return true;
            }
        }
        return false;
    }

    // ! Make this private eventually
    function PartialKeyRegistered(
        uint256 groupIndex,
        address nodeIdAddress,
        bytes memory partialKey
    ) public view returns (bool) {
        Group storage g = groups[groupIndex];
        for (uint256 i = 0; i < g.commitCache.length; i++) {
            if (
                g.commitCache[i].nodeIdAddress == nodeIdAddress &&
                keccak256(g.commitCache[i].partialPublicKey) ==
                keccak256(partialKey)
            ) {
                return true;
            }
        }
        return false;
    }

    function commitDkg(
        // address idAddress,
        uint256 groupIndex,
        uint256 groupEpoch,
        bytes calldata publicKey,
        bytes calldata partialPublicKey,
        address[] calldata disqualifiedNodes
    ) public {
        // // ! I added a check of idAddres = msg.sender, ask Ruoshan
        // require(
        //     idAddress == msg.sender,
        //     "Node id address does not match msg.sender"
        // );

        require(groupRegistered[groupIndex], "Group does not exist"); // require group exists
        // TODO: Bincode deserialize

        require(
            coordinators[groupIndex] != address(0),
            "Coordinator not found for groupIndex"
        ); // require coordinator exists

        // Ensure DKG Proccess is in Phase
        ICoordinator coordinator = ICoordinator(coordinators[groupIndex]);
        int8 phase = coordinator.inPhase(); // get current phase
        require(phase != -1, "DKG Has ended"); // require coordinator is in phase 1

        // Ensure Eopch is correct,  Node is in group, and has not already submitted a partial key
        Group storage g = groups[groupIndex]; // get group from group index
        require(
            groupEpoch == g.epoch,
            "Caller Group epoch does not match Controller Group epoch"
        );
        require(
            NodeInMembers(groupIndex, msg.sender),
            "Node is not a member of the group"
        );
        require(
            !PartialKeyRegistered(groupIndex, msg.sender, partialPublicKey),
            "CommitCache already contains PartialKey for this node"
        );

        // Populate CommitResult / CommitCache
        CommitResult memory commitResult = CommitResult({
            groupEpoch: groupEpoch,
            publicKey: publicKey,
            disqualifiedNodes: disqualifiedNodes
        });

        CommitCache memory commitCache = CommitCache({
            commitResult: commitResult,
            partialPublicKey: partialPublicKey,
            nodeIdAddress: msg.sender
        });

        g.commitCache.push(commitCache);

        // If consensus was reached previously...
        if (g.isStrictlyMajorityConsensusReached) {
            // assign member partial public keys
            for (uint256 i = 0; i < g.members.length; i++) {
                if (g.members[i].nodeIdAddress == msg.sender) {
                    g.members[i].partialPublicKey = partialPublicKey;
                }
            }
        } else {
            // check if consensus was just reached...
            (
                bool consensusReached,
                address[] memory majority_members
            ) = getStrictlyMajorityIdenticalCommitmentResult(groupIndex);

            if (consensusReached) {
                // TODO: let last_output = self.last_output as usize; // * What is this?
                // TODO: majority_members.retain(|m| !identical_commit.disqualified_nodes.contains(m));
                // TODO: ensure majority members aren't contained in disqualified nodes.
                if (majority_members.length > g.threshold) {
                    g.isStrictlyMajorityConsensusReached = true;
                    // g.size -= g.disqualifiedNodes.length;
                    // assign member partial public keys
                    for (uint256 i = 0; i < g.members.length; i++) {
                        for (uint256 j = 0; j < majority_members.length; j++) {
                            if (
                                g.members[i].nodeIdAddress ==
                                majority_members[j]
                            ) {
                                g
                                    .members[i]
                                    .partialPublicKey = partialPublicKey;
                            }
                        }
                    }
                }
            }

            // TODO: Draw the rest of the owl (line 870 in BLS Repo)
            // Qualified Indices / Commiter indices / CHoose randomly from indices
            // Move disqualified nodes out of group
        }
    }

    //! Ask Ruoshan, can this be done more gas efficintly?
    // groupIndex => commitResult (hashed) => Node Address Array
    mapping(uint256 => mapping(bytes32 => address[])) commitResultToNodes;
    mapping(uint256 => mapping(bytes32 => bool)) commitResultSeen; // keep track of commit results seen

    // Goal: get array of majority members with identical commit result
    function getStrictlyMajorityIdenticalCommitmentResult(uint256 groupIndex)
        internal
        returns (bool, address[] memory)
    {
        Group memory g = groups[groupIndex]; // get group from group index

        // Populate commitResultToNodes with identical commit results => node array
        for (uint256 i = 0; i < g.commitCache.length; i++) {
            CommitCache memory commitCache = g.commitCache[i];
            bytes32 commitResultHash = keccak256(
                abi.encode(commitCache.commitResult)
            );
            if (commitResultSeen[groupIndex][commitResultHash]) {
                commitResultToNodes[groupIndex][commitResultHash].push(
                    g.commitCache[i].nodeIdAddress
                );
            } else {
                commitResultSeen[groupIndex][commitResultHash] = true;
                commitResultToNodes[groupIndex][
                    commitResultHash
                ] = new address[](0);
                commitResultToNodes[groupIndex][commitResultHash].push(
                    g.commitCache[i].nodeIdAddress
                );
            }
        }

        // iterate through commitResultToNodes[groupIndex] and check if majority exists. If it does, return the nodes
        for (uint256 i = 0; i < g.commitCache.length; i++) {
            CommitCache memory commitCache = g.commitCache[i];
            bytes32 commitResultHash = keccak256(
                abi.encode(commitCache.commitResult)
            );
            if (
                commitResultToNodes[groupIndex][commitResultHash].length >
                g.members.length / 2 //! Ask ruoshan about this!
            ) {
                // g.isStrictlyMajorrityConsensusReached = true;
                return (
                    true,
                    commitResultToNodes[groupIndex][commitResultHash]
                );
            }
        }
        return (false, new address[](0));
    }

    // ! Post Proccess DKG

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
