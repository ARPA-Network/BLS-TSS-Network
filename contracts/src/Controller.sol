// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";
import "openzeppelin-contracts-upgradeable/contracts/proxy/utils/Initializable.sol";
import "./interfaces/IController.sol";
import "./interfaces/IControllerOwner.sol";
import "./interfaces/ICoordinator.sol";
import "Staking-v0.1/interfaces/INodeStaking.sol";
import {BLS} from "./libraries/BLS.sol";
import "./libraries/GroupLib.sol";
import {Coordinator} from "./Coordinator.sol";

contract Controller is Initializable, IController, IControllerOwner, OwnableUpgradeable {
    using SafeERC20 for IERC20;
    using GroupLib for GroupLib.GroupData;

    // *Controller Config*
    ControllerConfig private s_config;
    IERC20 private i_ARPA;

    // *Node State Variables*
    mapping(address => Node) private s_nodes; // maps node address to Node Struct
    mapping(address => uint256) private s_rewards; // maps node address to reward amount

    // *DKG Variables*
    mapping(uint256 => address) private s_coordinators; // maps group index to coordinator address

    // *Group Variables*
    GroupLib.GroupData s_groupData;

    // *Task Variables*
    uint256 private s_lastOutput;

    // *Structs*
    struct ControllerConfig {
        address stakingContractAddress;
        address adapterContractAddress;
        uint256 nodeStakingAmount;
        uint256 disqualifiedNodePenaltyAmount;
        uint256 defaultDkgPhaseDuration;
        uint256 pendingBlockAfterQuit;
        uint256 dkgPostProcessReward;
    }

    // *Events*
    event NodeRegistered(address indexed nodeAddress, bytes dkgPublicKey, uint256 groupIndex);
    event NodeActivated(address indexed nodeAddress, uint256 groupIndex);
    event NodeQuit(address indexed nodeAddress);
    event DkgPublicKeyChanged(address indexed nodeAddress, bytes dkgPublicKey);
    event NodeSlashed(address indexed nodeIdAddress, uint256 stakingRewardPenalty, uint256 pendingBlock);
    event NodeRewarded(address indexed nodeAddress, uint256 amount);
    event ControllerConfigSet(
        address stakingContractAddress,
        address adapterContractAddress,
        uint256 nodeStakingAmount,
        uint256 disqualifiedNodePenaltyAmount,
        uint256 defaultNumberOfCommitters,
        uint256 defaultDkgPhaseDuration,
        uint256 groupMaxCapacity,
        uint256 idealNumberOfGroups,
        uint256 pendingBlockAfterQuit,
        uint256 dkgPostProcessReward
    );
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

    // *Errors*
    error NodeNotRegistered();
    error NodeAlreadyRegistered();
    error NodeAlreadyActive();
    error NodeStillPending(uint256 pendingUntilBlock);
    error GroupNotExist(uint256 groupIndex);
    error CoordinatorNotFound(uint256 groupIndex);
    error DkgNotInProgress(uint256 groupIndex);
    error DkgStillInProgress(uint256 groupIndex, int8 phase);
    error EpochMismatch(uint256 groupIndex, uint256 inputGroupEpoch, uint256 currentGroupEpoch);
    error NodeNotInGroup(uint256 groupIndex, address nodeIdAddress);
    error PartialKeyAlreadyRegistered(uint256 groupIndex, address nodeIdAddress);
    error SenderNotAdapter();
    error InsufficientBalance();

    function initialize(address arpa, uint256 lastOutput) public initializer {
        i_ARPA = IERC20(arpa);
        s_lastOutput = lastOutput;

        __Ownable_init();
    }

    // =============
    // IControllerOwner
    // =============
    function setControllerConfig(
        address stakingContractAddress,
        address adapterContractAddress,
        uint256 nodeStakingAmount,
        uint256 disqualifiedNodePenaltyAmount,
        uint256 defaultNumberOfCommitters,
        uint256 defaultDkgPhaseDuration,
        uint256 groupMaxCapacity,
        uint256 idealNumberOfGroups,
        uint256 pendingBlockAfterQuit,
        uint256 dkgPostProcessReward
    ) external override(IControllerOwner) onlyOwner {
        s_config = ControllerConfig({
            stakingContractAddress: stakingContractAddress,
            adapterContractAddress: adapterContractAddress,
            nodeStakingAmount: nodeStakingAmount,
            disqualifiedNodePenaltyAmount: disqualifiedNodePenaltyAmount,
            defaultDkgPhaseDuration: defaultDkgPhaseDuration,
            pendingBlockAfterQuit: pendingBlockAfterQuit,
            dkgPostProcessReward: dkgPostProcessReward
        });

        s_groupData.setConfig(idealNumberOfGroups, groupMaxCapacity, defaultNumberOfCommitters);

        emit ControllerConfigSet(
            stakingContractAddress,
            adapterContractAddress,
            nodeStakingAmount,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            pendingBlockAfterQuit,
            dkgPostProcessReward
        );
    }

    // =============
    // IController
    // =============
    function nodeRegister(bytes calldata dkgPublicKey) external override(IController) {
        if (s_nodes[msg.sender].idAddress != address(0)) {
            revert NodeAlreadyRegistered();
        }

        uint256[4] memory publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
        if (!BLS.isValidPublicKey(publicKey)) {
            revert BLS.InvalidPublicKey();
        }
        // Lock staking amount in Staking contract
        INodeStaking(s_config.stakingContractAddress).lock(msg.sender, s_config.nodeStakingAmount);

        // Populate Node struct and insert into nodes
        Node storage n = s_nodes[msg.sender];
        n.idAddress = msg.sender;
        n.dkgPublicKey = dkgPublicKey;
        n.state = true;

        (uint256 groupIndex, uint256[] memory groupIndicesToEmitEvent) = s_groupData.nodeJoin(msg.sender, s_lastOutput);

        for (uint256 i = 0; i < groupIndicesToEmitEvent.length; i++) {
            emitGroupEvent(groupIndicesToEmitEvent[i]);
        }

        emit NodeRegistered(msg.sender, dkgPublicKey, groupIndex);
    }

    function nodeActivate() external override(IController) {
        Node storage node = s_nodes[msg.sender];
        if (node.idAddress != msg.sender) {
            revert NodeNotRegistered();
        }

        if (node.state) {
            revert NodeAlreadyActive();
        }

        if (node.pendingUntilBlock > block.number) {
            revert NodeStillPending(node.pendingUntilBlock);
        }

        // lock up to staking amount in Staking contract
        uint256 lockedAmount = INodeStaking(s_config.stakingContractAddress).getLockedAmount(msg.sender);
        if (lockedAmount < s_config.nodeStakingAmount) {
            INodeStaking(s_config.stakingContractAddress).lock(msg.sender, s_config.nodeStakingAmount - lockedAmount);
        }

        node.state = true;

        (uint256 groupIndex, uint256[] memory groupIndicesToEmitEvent) = s_groupData.nodeJoin(msg.sender, s_lastOutput);

        for (uint256 i = 0; i < groupIndicesToEmitEvent.length; i++) {
            emitGroupEvent(groupIndicesToEmitEvent[i]);
        }

        emit NodeActivated(msg.sender, groupIndex);
    }

    function nodeQuit() external override(IController) {
        Node storage node = s_nodes[msg.sender];

        if (node.idAddress != msg.sender) {
            revert NodeNotRegistered();
        }
        uint256[] memory groupIndicesToEmitEvent = s_groupData.nodeLeave(msg.sender, s_lastOutput);

        for (uint256 i = 0; i < groupIndicesToEmitEvent.length; i++) {
            emitGroupEvent(groupIndicesToEmitEvent[i]);
        }

        freezeNode(msg.sender, s_config.pendingBlockAfterQuit);

        // unlock staking amount in Staking contract
        INodeStaking(s_config.stakingContractAddress).unlock(msg.sender, s_config.nodeStakingAmount);

        emit NodeQuit(msg.sender);
    }

    function changeDkgPublicKey(bytes calldata dkgPublicKey) external override(IController) {
        Node storage node = s_nodes[msg.sender];
        if (node.idAddress != msg.sender) {
            revert NodeNotRegistered();
        }

        if (node.state) {
            revert NodeAlreadyActive();
        }

        uint256[4] memory publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
        if (!BLS.isValidPublicKey(publicKey)) {
            revert BLS.InvalidPublicKey();
        }

        node.dkgPublicKey = dkgPublicKey;

        emit DkgPublicKeyChanged(msg.sender, dkgPublicKey);
    }

    function commitDkg(CommitDkgParams memory params) external override(IController) {
        if (params.groupIndex >= s_groupData.s_groupCount) revert GroupNotExist(params.groupIndex);

        // require coordinator exists
        if (s_coordinators[params.groupIndex] == address(0)) {
            revert CoordinatorNotFound(params.groupIndex);
        }

        // Ensure DKG Proccess is in Phase
        ICoordinator coordinator = ICoordinator(s_coordinators[params.groupIndex]);
        if (coordinator.inPhase() == -1) {
            revert DkgNotInProgress(params.groupIndex);
        }

        // Ensure epoch is correct, node is in group, and has not already submitted a partial key
        Group storage g = s_groupData.s_groups[params.groupIndex];
        if (params.groupEpoch != g.epoch) {
            revert EpochMismatch(params.groupIndex, params.groupEpoch, g.epoch);
        }

        if (s_groupData.getMemberIndexByAddress(params.groupIndex, msg.sender) == -1) {
            revert NodeNotInGroup(params.groupIndex, msg.sender);
        }

        // check to see if member has called commitdkg in the past.
        if (isPartialKeyRegistered(params.groupIndex, msg.sender)) {
            revert PartialKeyAlreadyRegistered(params.groupIndex, msg.sender);
        }

        // require publickey and partial public key are not empty  / are the right format
        uint256[4] memory partialPublicKey = BLS.fromBytesPublicKey(params.partialPublicKey);
        if (!BLS.isValidPublicKey(partialPublicKey)) {
            revert BLS.InvalidPartialPublicKey();
        }

        uint256[4] memory publicKey = BLS.fromBytesPublicKey(params.publicKey);
        if (!BLS.isValidPublicKey(publicKey)) {
            revert BLS.InvalidPublicKey();
        }

        // Populate CommitResult / CommitCache
        CommitResult memory commitResult = CommitResult({
            groupEpoch: params.groupEpoch,
            publicKey: publicKey,
            disqualifiedNodes: params.disqualifiedNodes
        });

        if (!s_groupData.tryAddToExistingCommitCache(params.groupIndex, commitResult)) {
            CommitCache memory commitCache = CommitCache({commitResult: commitResult, nodeIdAddress: new address[](1)});

            commitCache.nodeIdAddress[0] = msg.sender;
            g.commitCacheList.push(commitCache);
        }

        // no matter consensus previously reached, update the partial public key of the given node's member entry in the group
        g.members[uint256(s_groupData.getMemberIndexByAddress(params.groupIndex, msg.sender))].partialPublicKey =
            partialPublicKey;

        // if not.. call get StrictlyMajorityIdenticalCommitmentResult for the group and check if consensus has been reached.
        if (!g.isStrictlyMajorityConsensusReached) {
            (bool success, address[] memory disqualifiedNodes) =
                s_groupData.tryEnableGroup(params.groupIndex, s_lastOutput);

            if (success) {
                // Iterate over disqualified nodes and call slashNode on each.
                for (uint256 i = 0; i < disqualifiedNodes.length; i++) {
                    slashNode(disqualifiedNodes[i], s_config.disqualifiedNodePenaltyAmount, 0);
                }
            }
        }
    }

    function postProcessDkg(uint256 groupIndex, uint256 groupEpoch) external override(IController) {
        if (groupIndex >= s_groupData.s_groupCount) revert GroupNotExist(groupIndex);

        // require calling node is in group
        if (s_groupData.getMemberIndexByAddress(groupIndex, msg.sender) == -1) {
            revert NodeNotInGroup(groupIndex, msg.sender);
        }

        // require correct epoch
        Group storage g = s_groupData.s_groups[groupIndex];
        if (groupEpoch != g.epoch) {
            revert EpochMismatch(groupIndex, groupEpoch, g.epoch);
        }

        // require coordinator exists
        if (s_coordinators[groupIndex] == address(0)) {
            revert CoordinatorNotFound(groupIndex);
        }

        // Ensure DKG Proccess is out of phase
        ICoordinator coordinator = ICoordinator(s_coordinators[groupIndex]);
        if (coordinator.inPhase() != -1) {
            revert DkgStillInProgress(groupIndex, coordinator.inPhase());
        }

        // delete coordinator
        coordinator.selfDestruct(); // coordinator self destructs
        s_coordinators[groupIndex] = address(0); // remove coordinator from mapping

        if (!g.isStrictlyMajorityConsensusReached) {
            (address[] memory nodesToBeSlashed, uint256[] memory groupIndicesToEmitEvent) =
                s_groupData.handleUnsuccessfulGroupDkg(groupIndex, s_lastOutput);

            for (uint256 i = 0; i < nodesToBeSlashed.length; i++) {
                slashNode(nodesToBeSlashed[i], s_config.disqualifiedNodePenaltyAmount, 0);
            }
            for (uint256 i = 0; i < groupIndicesToEmitEvent.length; i++) {
                emitGroupEvent(groupIndicesToEmitEvent[i]);
            }
        }

        // update rewards for calling node
        s_rewards[msg.sender] += s_config.dkgPostProcessReward;

        emit NodeRewarded(msg.sender, s_config.dkgPostProcessReward);
    }

    function claimReward(address recipient, uint256 amount) external override(IController) {
        if (s_rewards[msg.sender] < amount) {
            revert InsufficientBalance();
        }
        s_rewards[recipient] -= amount;
        i_ARPA.safeTransfer(recipient, amount);
    }

    function addReward(address[] memory nodes, uint256 amount) public override(IController) {
        if (msg.sender != s_config.adapterContractAddress) {
            revert SenderNotAdapter();
        }
        for (uint256 i = 0; i < nodes.length; i++) {
            s_rewards[nodes[i]] += amount;
            emit NodeRewarded(nodes[i], amount);
        }
    }

    function setLastOutput(uint256 lastOutput) external override(IController) {
        if (msg.sender != s_config.adapterContractAddress) {
            revert SenderNotAdapter();
        }
        s_lastOutput = lastOutput;
    }

    function getValidGroupIndices() public view override(IController) returns (uint256[] memory) {
        return s_groupData.getValidGroupIndices();
    }

    function getGroupCount() external view override(IController) returns (uint256) {
        return s_groupData.s_groupCount;
    }

    function getGroup(uint256 groupIndex) public view override(IController) returns (Group memory) {
        return s_groupData.s_groups[groupIndex];
    }

    function getNode(address nodeAddress) public view override(IController) returns (Node memory) {
        return s_nodes[nodeAddress];
    }

    function getMember(uint256 groupIndex, uint256 memberIndex)
        public
        view
        override(IController)
        returns (Member memory)
    {
        return s_groupData.s_groups[groupIndex].members[memberIndex];
    }

    function getBelongingGroup(address nodeAddress) external view override(IController) returns (int256, int256) {
        return s_groupData.getBelongingGroupByMemberAddress(nodeAddress);
    }

    function getCoordinator(uint256 groupIndex) public view override(IController) returns (address) {
        return s_coordinators[groupIndex];
    }

    function getNodeReward(address nodeAddress) public view override(IController) returns (uint256) {
        return s_rewards[nodeAddress];
    }

    function getLastOutput() external view returns (uint256) {
        return s_lastOutput;
    }

    /// Check to see if a group has a partial public key registered for a given node.
    function isPartialKeyRegistered(uint256 groupIndex, address nodeIdAddress)
        public
        view
        override(IController)
        returns (bool)
    {
        Group memory g = s_groupData.s_groups[groupIndex];
        for (uint256 i = 0; i < g.members.length; i++) {
            if (g.members[i].nodeIdAddress == nodeIdAddress) {
                return g.members[i].partialPublicKey[0] != 0;
            }
        }
        return false;
    }

    // =============
    // Internal
    // =============

    function emitGroupEvent(uint256 groupIndex) internal {
        s_groupData.prepareGroupEvent(groupIndex);

        Group memory g = s_groupData.s_groups[groupIndex];

        // Deploy coordinator, add to coordinators mapping
        Coordinator coordinator;
        coordinator = new Coordinator(g.threshold, s_config.defaultDkgPhaseDuration);
        s_coordinators[groupIndex] = address(coordinator);

        // Initialize Coordinator
        address[] memory groupNodes = new address[](g.size);
        bytes[] memory groupKeys = new bytes[](g.size);

        for (uint256 i = 0; i < g.size; i++) {
            groupNodes[i] = g.members[i].nodeIdAddress;
            groupKeys[i] = s_nodes[g.members[i].nodeIdAddress].dkgPublicKey;
        }

        coordinator.initialize(groupNodes, groupKeys);

        emit DkgTask(
            s_groupData.s_epoch, g.index, g.epoch, g.size, g.threshold, groupNodes, block.number, address(coordinator)
        );
    }

    // Give node staking reward penalty and freezeNode
    function slashNode(address nodeIdAddress, uint256 stakingRewardPenalty, uint256 pendingBlock) internal {
        // slash staking reward in Staking contract
        INodeStaking(s_config.stakingContractAddress).slashDelegationReward(nodeIdAddress, stakingRewardPenalty);

        // remove node from group if handleGroup is true and deactivate it
        freezeNode(nodeIdAddress, pendingBlock);

        emit NodeSlashed(nodeIdAddress, stakingRewardPenalty, pendingBlock);
    }

    function freezeNode(address nodeIdAddress, uint256 pendingBlock) internal {
        // set node state to false for frozen node
        s_nodes[nodeIdAddress].state = false;

        uint256 currentBlock = block.number;
        // if the node is already pending, add the pending block to the current pending block
        if (s_nodes[nodeIdAddress].pendingUntilBlock > currentBlock) {
            s_nodes[nodeIdAddress].pendingUntilBlock += pendingBlock;
            // else set the pending block to the current block + pending block
        } else {
            s_nodes[nodeIdAddress].pendingUntilBlock = currentBlock + pendingBlock;
        }
    }
}
