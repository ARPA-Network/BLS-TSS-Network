// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {OwnableUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/Initializable.sol";
import {IController} from "./interfaces/IController.sol";
import {IControllerOwner} from "./interfaces/IControllerOwner.sol";
import {IAdapter} from "./interfaces/IAdapter.sol";
import {ICoordinator} from "./interfaces/ICoordinator.sol";
import {INodeRegistry} from "./interfaces/INodeRegistry.sol";
import {BLS} from "./libraries/BLS.sol";
import {GroupLib} from "./libraries/GroupLib.sol";
import {Coordinator} from "./Coordinator.sol";

contract Controller is Initializable, IController, IControllerOwner, OwnableUpgradeable {
    using GroupLib for GroupLib.GroupData;

    // *Controller Config*
    ControllerConfig internal _config;

    // *DKG Variables*
    mapping(uint256 => address) internal _coordinators; // maps group index to coordinator address

    // *Group Variables*
    GroupLib.GroupData internal _groupData;

    // *Task Variables*
    uint256 internal _lastOutput;

    // *Structs*
    struct ControllerConfig {
        address nodeRegistryContractAddress;
        address adapterContractAddress;
        uint256 disqualifiedNodePenaltyAmount;
        uint256 defaultDkgPhaseDuration;
        uint256 dkgPostProcessReward;
    }

    // *Events*
    event ControllerConfigSet(
        address nodeRegistryContractAddress,
        address adapterContractAddress,
        uint256 disqualifiedNodePenaltyAmount,
        uint256 defaultNumberOfCommitters,
        uint256 defaultDkgPhaseDuration,
        uint256 groupMaxCapacity,
        uint256 idealNumberOfGroups,
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
    error GroupNotExist(uint256 groupIndex);
    error CoordinatorNotFound(uint256 groupIndex);
    error DkgNotInProgress(uint256 groupIndex);
    error DkgStillInProgress(uint256 groupIndex, int8 phase);
    error EpochMismatch(uint256 groupIndex, uint256 inputGroupEpoch, uint256 currentGroupEpoch);
    error NodeNotInGroup(uint256 groupIndex, address nodeIdAddress);
    error PartialKeyAlreadyRegistered(uint256 groupIndex, address nodeIdAddress);
    error SenderNotAdapter();
    error SenderNotNodeRegistry();
    error DuplicatedDisqualifiedNode();
    error CannotLeaveGroupDuringDkg();

    function initialize(uint256 lastOutput) public initializer {
        _lastOutput = lastOutput;

        __Ownable_init();
    }

    // =============
    // IControllerOwner
    // =============
    function setControllerConfig(
        address nodeRegistryContractAddress,
        address adapterContractAddress,
        uint256 disqualifiedNodePenaltyAmount,
        uint256 defaultNumberOfCommitters,
        uint256 defaultDkgPhaseDuration,
        uint256 groupMaxCapacity,
        uint256 idealNumberOfGroups,
        uint256 dkgPostProcessReward
    ) external override(IControllerOwner) onlyOwner {
        _config = ControllerConfig({
            nodeRegistryContractAddress: nodeRegistryContractAddress,
            adapterContractAddress: adapterContractAddress,
            disqualifiedNodePenaltyAmount: disqualifiedNodePenaltyAmount,
            defaultDkgPhaseDuration: defaultDkgPhaseDuration,
            dkgPostProcessReward: dkgPostProcessReward
        });

        _groupData.setConfig(idealNumberOfGroups, groupMaxCapacity, defaultNumberOfCommitters);

        emit ControllerConfigSet(
            nodeRegistryContractAddress,
            adapterContractAddress,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            dkgPostProcessReward
        );
    }

    // =============
    // IController
    // =============
    function nodeJoin(address nodeIdAddress) external override(IController) returns (uint256) {
        if (msg.sender != _config.nodeRegistryContractAddress) {
            revert SenderNotNodeRegistry();
        }

        (uint256 groupIndex, uint256[] memory groupIndicesToEmitEvent) = _groupData.nodeJoin(nodeIdAddress, _lastOutput);

        for (uint256 i = 0; i < groupIndicesToEmitEvent.length; i++) {
            _emitGroupEvent(groupIndicesToEmitEvent[i]);
        }

        return groupIndex;
    }

    function nodeLeave(address nodeIdAddress) external override(IController) {
        if (msg.sender != _config.nodeRegistryContractAddress) {
            revert SenderNotNodeRegistry();
        }

        (int256 groupIndex, int256 memberIndex) = _groupData.getBelongingGroupByMemberAddress(nodeIdAddress);

        if (groupIndex != -1) {
            if (_coordinators[uint256(groupIndex)] != address(0)) {
                revert CannotLeaveGroupDuringDkg();
            }

            uint256[] memory groupIndicesToEmitEvent =
                _groupData.nodeLeave(uint256(groupIndex), uint256(memberIndex), _lastOutput);

            for (uint256 i = 0; i < groupIndicesToEmitEvent.length; i++) {
                _emitGroupEvent(groupIndicesToEmitEvent[i]);
            }
        }
    }

    function commitDkg(CommitDkgParams memory params) external override(IController) {
        if (params.groupIndex >= _groupData.groupCount) {
            revert GroupNotExist(params.groupIndex);
        }

        // require coordinator exists
        if (_coordinators[params.groupIndex] == address(0)) {
            revert CoordinatorNotFound(params.groupIndex);
        }

        // Ensure DKG Proccess is in Phase
        ICoordinator coordinator = ICoordinator(_coordinators[params.groupIndex]);
        if (coordinator.inPhase() == -1) {
            revert DkgNotInProgress(params.groupIndex);
        }

        // Ensure epoch is correct, node is in group, and has not already submitted a partial key
        Group storage g = _groupData.groups[params.groupIndex];
        if (params.groupEpoch != g.epoch) {
            revert EpochMismatch(params.groupIndex, params.groupEpoch, g.epoch);
        }

        if (_groupData.getMemberIndexByAddress(params.groupIndex, msg.sender) == -1) {
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

        // no matter consensus previously reached, update the partial public key of the given node's member entry in the group
        g.members[uint256(_groupData.getMemberIndexByAddress(params.groupIndex, msg.sender))].partialPublicKey =
            partialPublicKey;

        // if not.. record commitResult, get StrictlyMajorityIdenticalCommitmentResult for the group and check if consensus has been reached.
        if (!g.isStrictlyMajorityConsensusReached) {
            // check if disqualifiedNodes is valid
            for (uint256 i = 0; i < params.disqualifiedNodes.length; i++) {
                if (_groupData.getMemberIndexByAddress(params.groupIndex, params.disqualifiedNodes[i]) == -1) {
                    revert NodeNotInGroup(params.groupIndex, params.disqualifiedNodes[i]);
                }
                for (uint256 j = i + 1; j < params.disqualifiedNodes.length; j++) {
                    if (params.disqualifiedNodes[i] == params.disqualifiedNodes[j]) {
                        revert DuplicatedDisqualifiedNode();
                    }
                }
            }

            // Populate CommitResult / CommitCache
            CommitResult memory commitResult = CommitResult({
                groupEpoch: params.groupEpoch,
                publicKey: publicKey,
                disqualifiedNodes: params.disqualifiedNodes
            });

            if (!_groupData.tryAddToExistingCommitCache(params.groupIndex, commitResult)) {
                CommitCache memory commitCache =
                    CommitCache({commitResult: commitResult, nodeIdAddress: new address[](1)});

                commitCache.nodeIdAddress[0] = msg.sender;
                g.commitCacheList.push(commitCache);
            }

            (bool success, address[] memory disqualifiedNodes) =
                _groupData.tryEnableGroup(params.groupIndex, _lastOutput);

            if (success) {
                // Iterate over disqualified nodes and call slashNode on each.
                for (uint256 i = 0; i < disqualifiedNodes.length; i++) {
                    INodeRegistry(_config.nodeRegistryContractAddress).slashNode(
                        disqualifiedNodes[i], _config.disqualifiedNodePenaltyAmount, 0
                    );
                }
            }
        }
    }

    function postProcessDkg(uint256 groupIndex, uint256 groupEpoch) external override(IController) {
        if (groupIndex >= _groupData.groupCount) {
            revert GroupNotExist(groupIndex);
        }

        // require calling node is in group
        if (_groupData.getMemberIndexByAddress(groupIndex, msg.sender) == -1) {
            revert NodeNotInGroup(groupIndex, msg.sender);
        }

        // require correct epoch
        Group storage g = _groupData.groups[groupIndex];
        if (groupEpoch != g.epoch) {
            revert EpochMismatch(groupIndex, groupEpoch, g.epoch);
        }

        // require coordinator exists
        if (_coordinators[groupIndex] == address(0)) {
            revert CoordinatorNotFound(groupIndex);
        }

        // Ensure DKG Proccess is out of phase
        ICoordinator coordinator = ICoordinator(_coordinators[groupIndex]);
        if (coordinator.inPhase() != -1) {
            revert DkgStillInProgress(groupIndex, coordinator.inPhase());
        }

        // delete coordinator
        coordinator.selfDestruct(); // coordinator self destructs
        _coordinators[groupIndex] = address(0); // remove coordinator from mapping

        if (!g.isStrictlyMajorityConsensusReached) {
            (address[] memory nodesToBeSlashed, uint256[] memory groupIndicesToEmitEvent) =
                _groupData.handleUnsuccessfulGroupDkg(groupIndex, _lastOutput);

            for (uint256 i = 0; i < nodesToBeSlashed.length; i++) {
                INodeRegistry(_config.nodeRegistryContractAddress).slashNode(
                    nodesToBeSlashed[i], _config.disqualifiedNodePenaltyAmount, 0
                );
            }
            for (uint256 i = 0; i < groupIndicesToEmitEvent.length; i++) {
                _emitGroupEvent(groupIndicesToEmitEvent[i]);
            }
        }

        // update rewards for calling node
        address[] memory nodeAddress = new address[](1);
        nodeAddress[0] = msg.sender;
        INodeRegistry(_config.nodeRegistryContractAddress).addReward(nodeAddress, 0, _config.dkgPostProcessReward);
    }

    function setLastOutput(uint256 lastOutput) external override(IController) {
        if (msg.sender != _config.adapterContractAddress) {
            revert SenderNotAdapter();
        }
        _lastOutput = lastOutput;
    }

    function addReward(address[] memory nodes, uint256 ethAmount, uint256 arpaAmount) external override(IController) {
        if (msg.sender != _config.adapterContractAddress) {
            revert SenderNotAdapter();
        }

        INodeRegistry(_config.nodeRegistryContractAddress).addReward(nodes, ethAmount, arpaAmount);
    }

    function nodeWithdrawETH(address recipient, uint256 ethAmount) external override(IController) {
        if (msg.sender != _config.nodeRegistryContractAddress) {
            revert SenderNotNodeRegistry();
        }
        IAdapter(_config.adapterContractAddress).nodeWithdrawETH(recipient, ethAmount);
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
            _config.nodeRegistryContractAddress,
            _config.adapterContractAddress,
            _config.disqualifiedNodePenaltyAmount,
            _groupData.defaultNumberOfCommitters,
            _config.defaultDkgPhaseDuration,
            _groupData.groupMaxCapacity,
            _groupData.idealNumberOfGroups,
            _config.dkgPostProcessReward
        );
    }

    function getValidGroupIndices() public view override(IController) returns (uint256[] memory) {
        return _groupData.getValidGroupIndices();
    }

    function getGroupEpoch() external view returns (uint256) {
        return _groupData.epoch;
    }

    function getGroupCount() external view override(IController) returns (uint256) {
        return _groupData.groupCount;
    }

    function getGroup(uint256 groupIndex) public view override(IController) returns (Group memory) {
        return _groupData.groups[groupIndex];
    }

    function getGroupThreshold(uint256 groupIndex) public view override(IController) returns (uint256, uint256) {
        return (_groupData.groups[groupIndex].threshold, _groupData.groups[groupIndex].size);
    }

    function getMember(uint256 groupIndex, uint256 memberIndex)
        public
        view
        override(IController)
        returns (Member memory)
    {
        return _groupData.groups[groupIndex].members[memberIndex];
    }

    function getBelongingGroup(address nodeAddress) external view override(IController) returns (int256, int256) {
        return _groupData.getBelongingGroupByMemberAddress(nodeAddress);
    }

    function getCoordinator(uint256 groupIndex) public view override(IController) returns (address) {
        return _coordinators[groupIndex];
    }

    function getLastOutput() external view returns (uint256) {
        return _lastOutput;
    }

    /// Check to see if a group has a partial public key registered for a given node.
    function isPartialKeyRegistered(uint256 groupIndex, address nodeIdAddress)
        public
        view
        override(IController)
        returns (bool)
    {
        Group memory g = _groupData.groups[groupIndex];
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

    function _emitGroupEvent(uint256 groupIndex) internal {
        _groupData.prepareGroupEvent(groupIndex);

        Group memory g = _groupData.groups[groupIndex];

        // Deploy coordinator, add to coordinators mapping
        Coordinator coordinator;
        coordinator = new Coordinator(g.threshold, _config.defaultDkgPhaseDuration);
        _coordinators[groupIndex] = address(coordinator);

        // Initialize Coordinator
        address[] memory groupNodes = new address[](g.size);
        bytes[] memory groupKeys = new bytes[](g.size);

        for (uint256 i = 0; i < g.size; i++) {
            groupNodes[i] = g.members[i].nodeIdAddress;
            // get node's dkg public key
            bytes memory dkgPublicKey =
                INodeRegistry(_config.nodeRegistryContractAddress).getDKGPublicKey(g.members[i].nodeIdAddress);
            groupKeys[i] = dkgPublicKey;
        }

        coordinator.initialize(groupNodes, groupKeys);

        emit DkgTask(
            _groupData.epoch, g.index, g.epoch, g.size, g.threshold, groupNodes, block.number, address(coordinator)
        );
    }
}
