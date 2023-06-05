// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

pragma experimental ABIEncoderV2;

import {ICoordinator} from "../src/interfaces/ICoordinator.sol";
import {RandcastTestHelper, IController, Controller, ControllerForTest, ERC20} from "./RandcastTestHelper.sol";
import {BLS} from "../src/libraries/BLS.sol";

// Suggested usage: forge test --match-contract ControllerTest --optimize -vv

contract ControllerTest is RandcastTestHelper {
    uint256 internal _disqualifiedNodePenaltyAmount = 1000;
    uint256 internal _defaultNumberOfCommitters = 3;
    uint256 internal _defaultDkgPhaseDuration = 10;
    uint256 internal _groupMaxCapacity = 10;
    uint256 internal _idealNumberOfGroups = 5;
    uint256 internal _pendingBlockAfterQuit = 100;
    uint256 internal _dkgPostProcessReward = 100;
    uint64 internal _lastOutput = 0x2222222222222222;

    address internal _owner = _admin;

    function setUp() public {
        // deal nodes
        vm.deal(_node1, 1 * 10 ** 18);
        vm.deal(_node2, 1 * 10 ** 18);
        vm.deal(_node3, 1 * 10 ** 18);
        vm.deal(_node4, 1 * 10 ** 18);
        vm.deal(_node5, 1 * 10 ** 18);
        vm.deal(_node6, 1 * 10 ** 18);
        vm.deal(_node7, 1 * 10 ** 18);
        vm.deal(_node8, 1 * 10 ** 18);
        vm.deal(_node9, 1 * 10 ** 18);
        vm.deal(_node10, 1 * 10 ** 18);
        vm.deal(_node11, 1 * 10 ** 18);

        // deal owner and create _controller
        vm.deal(_owner, 1 * 10 ** 18);
        vm.deal(_stakingDeployer, 1 * 10 ** 18);

        vm.prank(_owner);
        _arpa = new ERC20("arpa token", "ARPA");

        address[] memory operators = new address[](11);
        operators[0] = _node1;
        operators[1] = _node2;
        operators[2] = _node3;
        operators[3] = _node4;
        operators[4] = _node5;
        operators[5] = _node6;
        operators[6] = _node7;
        operators[7] = _node8;
        operators[8] = _node9;
        operators[9] = _node10;
        operators[10] = _node11;
        _prepareStakingContract(_stakingDeployer, address(_arpa), operators);

        vm.prank(_owner);
        _controller = new ControllerForTest(address(_arpa), _lastOutput);

        vm.prank(_owner);
        _controller.setControllerConfig(
            address(_staking),
            address(0),
            _operatorStakeAmount,
            _disqualifiedNodePenaltyAmount,
            _defaultNumberOfCommitters,
            _defaultDkgPhaseDuration,
            _groupMaxCapacity,
            _idealNumberOfGroups,
            _pendingBlockAfterQuit,
            _dkgPostProcessReward
        );

        vm.prank(_stakingDeployer);
        _staking.setController(address(_controller));
    }

    function testNodeRegister() public {
        // Fail on bad dkg public key
        vm.expectRevert(abi.encodeWithSelector(BLS.InvalidPublicKey.selector));
        vm.prank(_node1);
        _controller.nodeRegister(_badKey);

        // Register _node1
        vm.prank(_node1);
        _controller.nodeRegister(_dkgPubkey1);

        // Assert _node1 state is correct
        IController.Node memory n = _controller.getNode(_node1);
        assertEq(n.idAddress, _node1);
        assertEq(n.dkgPublicKey, _dkgPubkey1);
        assertEq(n.state, true);
        assertEq(n.pendingUntilBlock, 0);

        // fail on already registered node
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeAlreadyRegistered.selector));
        vm.prank(_node1);
        _controller.nodeRegister(_dkgPubkey1);
    }

    function testRemoveFromGroup() public {
        testCommitDkg();
        printGroupInfo(0);
        assertEq(_controller.getGroup(0).size, 3);
        _controller.removeFromGroupForTest(0, 0);
        printGroupInfo(0);
        assertEq(_controller.getGroup(0).size, 2);
    }

    function testRebalanceGroup() public {
        emit log_named_uint("groupCount", _controller.getGroupCount());
        testCommitDkg();
        emit log_named_uint("groupCount", _controller.getGroupCount());
        printGroupInfo(0);

        // Add 4th node, should create new group
        vm.prank(_node4);
        _controller.nodeRegister(_dkgPubkey4);
        emit log_named_uint("groupCount", _controller.getGroupCount());
        printGroupInfo(1);

        // The below needs further testing
        // Test needsRebalance
        vm.prank(_node5);
        _controller.nodeRegister(_dkgPubkey5);
        vm.prank(_node6);
        _controller.nodeRegister(_dkgPubkey6);
        vm.prank(_node7);
        _controller.nodeRegister(_dkgPubkey7);
        vm.prank(_node8);
        _controller.nodeRegister(_dkgPubkey8);
        vm.prank(_node9);
        _controller.nodeRegister(_dkgPubkey9);
        vm.prank(_node10);
        _controller.nodeRegister(_dkgPubkey10);
        vm.prank(_node11);
        _controller.nodeRegister(_dkgPubkey11);
        emit log("+++++++++++++++++++++++");
        printGroupInfo(0);
        printGroupInfo(1);
        emit log("++++++ Rebalance 1 +++++++");
        bool output = _controller.rebalanceGroupForTest(0, 1);
        assertEq(output, true);
        assertEq(_controller.getGroup(0).size, 5);
        assertEq(_controller.getGroup(1).size, 6);
        printGroupInfo(0);
        printGroupInfo(1);
        emit log("++++++ Rebalance 2 +++++++");
        output = _controller.rebalanceGroupForTest(0, 1);
        assertEq(output, true);
        assertEq(_controller.getGroup(0).size, 6);
        assertEq(_controller.getGroup(1).size, 5);
        printGroupInfo(0);
        printGroupInfo(1);
    }

    function testMinimumThreshold() public {
        uint256 min;
        min = _controller.minimumThresholdForTest(3);
        emit log_named_uint("min 3", min);
        assertEq(min, 2);
        min = _controller.minimumThresholdForTest(7);
        emit log_named_uint("min 7", min);
        assertEq(min, 4);
        min = _controller.minimumThresholdForTest(100);
        emit log_named_uint("min 100", min);
        assertEq(min, 51);
    }

    function testEmitGroupEvent() public {
        // * Register Three nodes and see if group struct is well formed
        uint256 groupIndex = 0;
        // printGroupInfo(groupIndex);
        // printNodeInfo(_node1);

        // Register Node 1
        vm.prank(_node1);
        _controller.nodeRegister(_dkgPubkey1);
        // printGroupInfo(groupIndex);
        // printNodeInfo(_node1);

        // Register Node 2
        vm.prank(_node2);
        _controller.nodeRegister(_dkgPubkey2);
        // printGroupInfo(groupIndex);

        // Register Node 3
        vm.prank(_node3);
        _controller.nodeRegister(_dkgPubkey3);
        // printGroupInfo(groupIndex);

        // check group struct is correct
        IController.Group memory g = _controller.getGroup(groupIndex);
        assertEq(g.index, 0);
        assertEq(g.epoch, 1);
        assertEq(g.size, 3);
        assertEq(g.threshold, 3);
        assertEq(g.members.length, 3);

        // Verify _node2 info is recorded in group.members[1]
        IController.Member memory m = g.members[1];
        // printMemberInfo(groupIndex, 1);
        assertEq(m.nodeIdAddress, _node2);
        // assertEq(m.partialPublicKey, TODO);

        // address coordinatorAddress = _controller.getCoordinator(groupIndex);
        // emit log_named_address("\nCoordinator", coordinatorAddress);
    }

    function testValidGroupIndices() public {
        uint256[] memory groupIndices = _controller.getValidGroupIndices();
        assertEq(groupIndices.length, 0);
        assertEq(_controller.getGroupCount(), 0);

        testCommitDkg();

        groupIndices = _controller.getValidGroupIndices();
        // for (uint256 i = 0; i < groupIndices.length; i++) {
        //     emit log_named_uint("groupIndices[i]", groupIndices[i]);
        // }
        assertEq(groupIndices.length, 1);
        assertEq(_controller.getGroupCount(), 1);
    }

    function testFindOrCreateTargetGroup() public {
        emit log_named_uint("groupCount", _controller.getGroupCount());
        testCommitDkg();
        emit log_named_uint("groupCount", _controller.getGroupCount());
        printGroupInfo(1);

        // Add 4th node, should create new group
        vm.prank(_node4);
        _controller.nodeRegister(_dkgPubkey4);
        emit log_named_uint("groupCount", _controller.getGroupCount());
        printGroupInfo(2);
    }

    function testGetMemberIndexByAddress() public {
        uint256 groupIndex = 0;

        int256 memberIndex = _controller.getMemberIndexByAddressForTest(groupIndex, _node1);
        assertEq(memberIndex, -1);

        testEmitGroupEvent();

        memberIndex = _controller.getMemberIndexByAddressForTest(groupIndex, _node1);
        assertEq(memberIndex, 0);
        memberIndex = _controller.getMemberIndexByAddressForTest(groupIndex, _node2);
        assertEq(memberIndex, 1);
        memberIndex = _controller.getMemberIndexByAddressForTest(groupIndex, _node3);
        assertEq(memberIndex, 2);
    }

    function testCoordinatorPhase() public {
        testEmitGroupEvent();
        uint256 groupIndex = 0;
        address coordinatorAddress = _controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();
        assertEq(coordinator.inPhase(), 1);
        vm.roll(startBlock + 1 + _defaultDkgPhaseDuration);
        assertEq(coordinator.inPhase(), 2);
        vm.roll(startBlock + 1 + 2 * _defaultDkgPhaseDuration);
        assertEq(coordinator.inPhase(), 3);
        vm.roll(startBlock + 1 + 3 * _defaultDkgPhaseDuration);
        assertEq(coordinator.inPhase(), 4);
        vm.roll(startBlock + 1 + 4 * _defaultDkgPhaseDuration);
        assertEq(coordinator.inPhase(), -1);
    }

    // Start commitdkg testing
    struct CommitDkgParams {
        uint256 groupIndex;
        uint256 groupEpoch;
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
    }

    function testCommitDkg() public {
        testEmitGroupEvent();

        uint256 groupIndex = 0;
        uint256 groupEpoch = 1;
        bytes memory partialPublicKey = _partialPublicKey1;
        bytes memory publicKey = _publicKey;
        address[] memory disqualifiedNodes = new address[](0);

        // Fail if group does not exist
        vm.prank(_node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.GroupNotExist.selector, 999));
        IController.CommitDkgParams memory params = IController.CommitDkgParams(
            999, // wrong group index
            groupEpoch,
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        _controller.commitDkg(params);

        // Fail if group does not match Controller Group Epoch
        vm.prank(_node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.EpochMismatch.selector, groupIndex, 999, groupEpoch));
        params = IController.CommitDkgParams(
            groupIndex,
            999, //  wrong epoch
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        _controller.commitDkg(params);

        // Fail if node is not a member of the group
        vm.prank(_node5);
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeNotInGroup.selector, groupIndex, address(_node5)));
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        _controller.commitDkg(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        // Succesful Commit: Node 1
        vm.prank(_node1);
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        _controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        //  Fail if CommitCache already contains PartialKey for this node
        vm.prank(_node1);
        vm.expectRevert(
            abi.encodeWithSelector(Controller.PartialKeyAlreadyRegistered.selector, groupIndex, address(_node1))
        );
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        _controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);

        // Succesful Commit: Node 2
        vm.prank(_node2);
        params = IController.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            _partialPublicKey2, // partial public key 2
            disqualifiedNodes
        );
        _controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        // Succesful Commit: Node 3
        vm.prank(_node3);
        params = IController.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            _partialPublicKey3, // partial public key 3
            disqualifiedNodes
        );
        _controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), true);
        // printGroupInfo(groupIndex);
    }

    function testPickRandomIndex() public {
        uint256[] memory indices = new uint256[](5);
        indices[0] = 0;
        indices[1] = 1;
        indices[2] = 2;
        indices[3] = 3;
        indices[4] = 4;

        uint256[] memory chosenIndices = _controller.pickRandomIndexForTest(_lastOutput, indices, 3);

        for (uint256 i = 0; i < chosenIndices.length; i++) {
            emit log_named_uint("chosenIndices", chosenIndices[i]);
        }

        assertEq(chosenIndices.length, 3);
    }

    function testGetNonDisqualifiedMajorityMembers() public {
        address[] memory nodes = new address[](3);
        nodes[0] = _node1;
        nodes[1] = _node2;
        nodes[2] = _node3;

        address[] memory disqualifedNodes = new address[](1);
        disqualifedNodes[0] = _node2;

        address[] memory majorityMembers = _controller.getNonDisqualifiedMajorityMembersForTest(nodes, disqualifedNodes);

        assertEq(majorityMembers.length, 2);
    }

    function testIsPartialKeyRegistered() public {
        testEmitGroupEvent();
        uint256 groupIndex = 1;
        assertEq(_controller.isPartialKeyRegistered(groupIndex, _node1), false);
    }

    function testPostProcessDkg() public {
        testCommitDkg();

        uint256 groupIndex = 0;
        uint256 groupEpoch = 1;
        address coordinatorAddress = _controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();

        vm.expectRevert(abi.encodeWithSelector(Controller.GroupNotExist.selector, 99999));
        _controller.postProcessDkg(99999, 0); //(groupIndex, groupEpoch))

        vm.prank(_node12);
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeNotInGroup.selector, groupIndex, _node12));
        _controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(_node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.EpochMismatch.selector, groupIndex, 0, groupEpoch));

        _controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(_node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.DkgStillInProgress.selector, groupIndex, 1));
        _controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))

        // Set the coordinator to completed phase
        vm.roll(startBlock + 1 + 4 * _defaultDkgPhaseDuration); // Put the coordinator in phase

        // Succesful post process dkg: HAPPY PATH
        vm.startPrank(_node1);
        _controller.postProcessDkg(groupIndex, groupEpoch);
        (, uint256 nodeArpaRewards) = _controller.getNodeWithdrawableTokens(_node1);
        emit log_named_uint("_node1 rewards", nodeArpaRewards);
        assertEq(nodeArpaRewards, _dkgPostProcessReward);

        // test self destruct worked properly
        address emptyCoordinatorAddress = _controller.getCoordinator(groupIndex);
        assertEq(emptyCoordinatorAddress, address(0));

        vm.expectRevert(abi.encodeWithSelector(Controller.CoordinatorNotFound.selector, groupIndex));
        _controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))
        vm.stopPrank();
        // assert that coordinator has self destructed (cant test this yet)
    }

    function testSlashNode() public {
        testPostProcessDkg();

        uint256 node1DelegationRewardBefore = _staking.getDelegationReward(_node1);
        emit log_named_uint("The delegation reward of _node1 before slash", node1DelegationRewardBefore);
        // slash _node1
        uint256 pendingBlock = 0;

        _controller.slashNodeForTest(_node1, _disqualifiedNodePenaltyAmount, pendingBlock);

        // Assert _staking penalty applied to _node1
        emit log_named_uint("The delegation reward of _node1 after slash", _staking.getDelegationReward(_node1));
        assertEq(node1DelegationRewardBefore - _disqualifiedNodePenaltyAmount, _staking.getDelegationReward(_node1));
    }

    function testNodeQuit() public {
        // call nodeQuit with unregistered node: Fail
        vm.prank(_node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeNotRegistered.selector));
        _controller.nodeQuit();

        // register node, confirm initial stake amount
        vm.prank(_node1);
        _controller.nodeRegister(_dkgPubkey1);
        assertEq(1, _controller.getGroup(0).members.length);
        // TODO
        // assertEq(operatorStakeAmount, staker.getLockedAmount(_node1));
        printGroupInfo(0);
        // printNodeInfo(_node1);

        // Quit node: Success
        vm.prank(_node1);
        _controller.nodeQuit();
        // assert member length is 0
        assertEq(0, _controller.getGroup(0).members.length);
        // TODO
        // assertEq(0, staker.getLockedAmount(_node1));
        // printGroupInfo(0);
        // printNodeInfo(_node1);
    }
}
