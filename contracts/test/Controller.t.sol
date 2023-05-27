// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

pragma experimental ABIEncoderV2;

import {ICoordinator} from "../src/interfaces/ICoordinator.sol";
import {RandcastTestHelper, IController, Controller, ControllerForTest, ERC20} from "./RandcastTestHelper.sol";
import {BLS} from "../src/libraries/BLS.sol";

// Suggested usage: forge test --match-contract ControllerTest --optimize -vv

contract ControllerTest is RandcastTestHelper {
    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 10;
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;
    uint64 lastOutput = 0x2222222222222222;

    address public owner = admin;

    function setUp() public {
        // deal nodes
        vm.deal(node1, 1 * 10 ** 18);
        vm.deal(node2, 1 * 10 ** 18);
        vm.deal(node3, 1 * 10 ** 18);
        vm.deal(node4, 1 * 10 ** 18);
        vm.deal(node5, 1 * 10 ** 18);
        vm.deal(node6, 1 * 10 ** 18);
        vm.deal(node7, 1 * 10 ** 18);
        vm.deal(node8, 1 * 10 ** 18);
        vm.deal(node9, 1 * 10 ** 18);
        vm.deal(node10, 1 * 10 ** 18);
        vm.deal(node11, 1 * 10 ** 18);

        // deal owner and create controller
        vm.deal(owner, 1 * 10 ** 18);
        vm.deal(stakingDeployer, 1 * 10 ** 18);

        vm.prank(owner);
        arpa = new ERC20("arpa token", "ARPA");

        address[] memory operators = new address[](11);
        operators[0] = node1;
        operators[1] = node2;
        operators[2] = node3;
        operators[3] = node4;
        operators[4] = node5;
        operators[5] = node6;
        operators[6] = node7;
        operators[7] = node8;
        operators[8] = node9;
        operators[9] = node10;
        operators[10] = node11;
        _prepareStakingContract(stakingDeployer, address(arpa), operators);

        vm.prank(owner);
        controller = new ControllerForTest(address(arpa), lastOutput);

        vm.prank(owner);
        controller.setControllerConfig(
            address(staking),
            address(0),
            operatorStakeAmount,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            pendingBlockAfterQuit,
            dkgPostProcessReward
        );

        vm.prank(stakingDeployer);
        staking.setController(address(controller));
    }

    function testNodeRegister() public {
        // Fail on bad dkg public key
        vm.expectRevert(abi.encodeWithSelector(BLS.InvalidPublicKey.selector));
        vm.prank(node1);
        controller.nodeRegister(badKey);

        // Register node1
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);

        // Assert node1 state is correct
        IController.Node memory n = controller.getNode(node1);
        assertEq(n.idAddress, node1);
        assertEq(n.dkgPublicKey, DKGPubkey1);
        assertEq(n.state, true);
        assertEq(n.pendingUntilBlock, 0);

        // fail on already registered node
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeAlreadyRegistered.selector));
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);
    }

    function testRemoveFromGroup() public {
        testCommitDkg();
        printGroupInfo(0);
        assertEq(controller.getGroup(0).size, 3);
        controller.removeFromGroupForTest(0, 0);
        printGroupInfo(0);
        assertEq(controller.getGroup(0).size, 2);
    }

    function testRebalanceGroup() public {
        emit log_named_uint("groupCount", controller.getGroupCount());
        testCommitDkg();
        emit log_named_uint("groupCount", controller.getGroupCount());
        printGroupInfo(0);

        // Add 4th node, should create new group
        vm.prank(node4);
        controller.nodeRegister(DKGPubkey4);
        emit log_named_uint("groupCount", controller.getGroupCount());
        printGroupInfo(1);

        // The below needs further testing
        // Test needsRebalance
        vm.prank(node5);
        controller.nodeRegister(DKGPubkey5);
        vm.prank(node6);
        controller.nodeRegister(DKGPubkey6);
        vm.prank(node7);
        controller.nodeRegister(DKGPubkey7);
        vm.prank(node8);
        controller.nodeRegister(DKGPubkey8);
        vm.prank(node9);
        controller.nodeRegister(DKGPubkey9);
        vm.prank(node10);
        controller.nodeRegister(DKGPubkey10);
        vm.prank(node11);
        controller.nodeRegister(DKGPubkey11);
        emit log("+++++++++++++++++++++++");
        printGroupInfo(0);
        printGroupInfo(1);
        emit log("++++++ Rebalance 1 +++++++");
        bool output = controller.rebalanceGroupForTest(0, 1);
        assertEq(output, true);
        assertEq(controller.getGroup(0).size, 5);
        assertEq(controller.getGroup(1).size, 6);
        printGroupInfo(0);
        printGroupInfo(1);
        emit log("++++++ Rebalance 2 +++++++");
        output = controller.rebalanceGroupForTest(0, 1);
        assertEq(output, true);
        assertEq(controller.getGroup(0).size, 6);
        assertEq(controller.getGroup(1).size, 5);
        printGroupInfo(0);
        printGroupInfo(1);
    }

    function testMinimumThreshold() public {
        uint256 min;
        min = controller.minimumThresholdForTest(3);
        emit log_named_uint("min 3", min);
        assertEq(min, 2);
        min = controller.minimumThresholdForTest(7);
        emit log_named_uint("min 7", min);
        assertEq(min, 4);
        min = controller.minimumThresholdForTest(100);
        emit log_named_uint("min 100", min);
        assertEq(min, 51);
    }

    function testEmitGroupEvent() public {
        // * Register Three nodes and see if group struct is well formed
        uint256 groupIndex = 0;
        // printGroupInfo(groupIndex);
        // printNodeInfo(node1);

        // Register Node 1
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);
        // printGroupInfo(groupIndex);
        // printNodeInfo(node1);

        // Register Node 2
        vm.prank(node2);
        controller.nodeRegister(DKGPubkey2);
        // printGroupInfo(groupIndex);

        // Register Node 3
        vm.prank(node3);
        controller.nodeRegister(DKGPubkey3);
        // printGroupInfo(groupIndex);

        // check group struct is correct
        IController.Group memory g = controller.getGroup(groupIndex);
        assertEq(g.index, 0);
        assertEq(g.epoch, 1);
        assertEq(g.size, 3);
        assertEq(g.threshold, 3);
        assertEq(g.members.length, 3);

        // Verify node2 info is recorded in group.members[1]
        IController.Member memory m = g.members[1];
        // printMemberInfo(groupIndex, 1);
        assertEq(m.nodeIdAddress, node2);
        // assertEq(m.partialPublicKey, TODO);

        // address coordinatorAddress = controller.getCoordinator(groupIndex);
        // emit log_named_address("\nCoordinator", coordinatorAddress);
    }

    function testValidGroupIndices() public {
        uint256[] memory groupIndices = controller.getValidGroupIndices();
        assertEq(groupIndices.length, 0);
        assertEq(controller.getGroupCount(), 0);

        testCommitDkg();

        groupIndices = controller.getValidGroupIndices();
        // for (uint256 i = 0; i < groupIndices.length; i++) {
        //     emit log_named_uint("groupIndices[i]", groupIndices[i]);
        // }
        assertEq(groupIndices.length, 1);
        assertEq(controller.getGroupCount(), 1);
    }

    function testFindOrCreateTargetGroup() public {
        emit log_named_uint("groupCount", controller.getGroupCount());
        testCommitDkg();
        emit log_named_uint("groupCount", controller.getGroupCount());
        printGroupInfo(1);

        // Add 4th node, should create new group
        vm.prank(node4);
        controller.nodeRegister(DKGPubkey4);
        emit log_named_uint("groupCount", controller.getGroupCount());
        printGroupInfo(2);
    }

    function testGetMemberIndexByAddress() public {
        uint256 groupIndex = 0;

        int256 memberIndex = controller.getMemberIndexByAddressForTest(groupIndex, node1);
        assertEq(memberIndex, -1);

        testEmitGroupEvent();

        memberIndex = controller.getMemberIndexByAddressForTest(groupIndex, node1);
        assertEq(memberIndex, 0);
        memberIndex = controller.getMemberIndexByAddressForTest(groupIndex, node2);
        assertEq(memberIndex, 1);
        memberIndex = controller.getMemberIndexByAddressForTest(groupIndex, node3);
        assertEq(memberIndex, 2);
    }

    function testCoordinatorPhase() public {
        testEmitGroupEvent();
        uint256 groupIndex = 0;
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();
        assertEq(coordinator.inPhase(), 1);
        vm.roll(startBlock + 1 + defaultDkgPhaseDuration);
        assertEq(coordinator.inPhase(), 2);
        vm.roll(startBlock + 1 + 2 * defaultDkgPhaseDuration);
        assertEq(coordinator.inPhase(), 3);
        vm.roll(startBlock + 1 + 3 * defaultDkgPhaseDuration);
        assertEq(coordinator.inPhase(), 4);
        vm.roll(startBlock + 1 + 4 * defaultDkgPhaseDuration);
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
        bytes memory partialPublicKey = partialPublicKey1;
        bytes memory publicKey = publicKey;
        address[] memory disqualifiedNodes = new address[](0);

        // Fail if group does not exist
        vm.prank(node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.GroupNotExist.selector, 999));
        IController.CommitDkgParams memory params = IController.CommitDkgParams(
            999, // wrong group index
            groupEpoch,
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        controller.commitDkg(params);

        // Fail if group does not match Controller Group Epoch
        vm.prank(node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.EpochMismatch.selector, groupIndex, 999, groupEpoch));
        params = IController.CommitDkgParams(
            groupIndex,
            999, //  wrong epoch
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        controller.commitDkg(params);

        // Fail if node is not a member of the group
        vm.prank(node5);
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeNotInGroup.selector, groupIndex, address(node5)));
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        controller.commitDkg(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        // Succesful Commit: Node 1
        vm.prank(node1);
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        //  Fail if CommitCache already contains PartialKey for this node
        vm.prank(node1);
        vm.expectRevert(
            abi.encodeWithSelector(Controller.PartialKeyAlreadyRegistered.selector, groupIndex, address(node1))
        );
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);

        // Succesful Commit: Node 2
        vm.prank(node2);
        params = IController.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey2, // partial public key 2
            disqualifiedNodes
        );
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        // Succesful Commit: Node 3
        vm.prank(node3);
        params = IController.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey3, // partial public key 3
            disqualifiedNodes
        );
        controller.commitDkg(params);
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

        uint256[] memory chosenIndices = controller.pickRandomIndexForTest(lastOutput, indices, 3);

        for (uint256 i = 0; i < chosenIndices.length; i++) {
            emit log_named_uint("chosenIndices", chosenIndices[i]);
        }

        assertEq(chosenIndices.length, 3);
    }

    function testGetNonDisqualifiedMajorityMembers() public {
        address[] memory nodes = new address[](3);
        nodes[0] = node1;
        nodes[1] = node2;
        nodes[2] = node3;

        address[] memory disqualifedNodes = new address[](1);
        disqualifedNodes[0] = node2;

        address[] memory majorityMembers = controller.getNonDisqualifiedMajorityMembersForTest(nodes, disqualifedNodes);

        assertEq(majorityMembers.length, 2);
    }

    function testIsPartialKeyRegistered() public {
        testEmitGroupEvent();
        uint256 groupIndex = 1;
        assertEq(controller.isPartialKeyRegistered(groupIndex, node1), false);
    }

    function testPostProcessDkg() public {
        testCommitDkg();

        uint256 groupIndex = 0;
        uint256 groupEpoch = 1;
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();

        vm.expectRevert(abi.encodeWithSelector(Controller.GroupNotExist.selector, 99999));
        controller.postProcessDkg(99999, 0); //(groupIndex, groupEpoch))

        vm.prank(node12);
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeNotInGroup.selector, groupIndex, node12));
        controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.EpochMismatch.selector, groupIndex, 0, groupEpoch));

        controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.DkgStillInProgress.selector, groupIndex, 1));
        controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))

        // Set the coordinator to completed phase
        vm.roll(startBlock + 1 + 4 * defaultDkgPhaseDuration); // Put the coordinator in phase

        // Succesful post process dkg: HAPPY PATH
        vm.startPrank(node1);
        controller.postProcessDkg(groupIndex, groupEpoch);
        (, uint256 nodeArpaRewards) = controller.getNodeWithdrawableTokens(node1);
        emit log_named_uint("node1 rewards", nodeArpaRewards);
        assertEq(nodeArpaRewards, dkgPostProcessReward);

        // test self destruct worked properly
        address emptyCoordinatorAddress = controller.getCoordinator(groupIndex);
        assertEq(emptyCoordinatorAddress, address(0));

        vm.expectRevert(abi.encodeWithSelector(Controller.CoordinatorNotFound.selector, groupIndex));
        controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))
        vm.stopPrank();
        // assert that coordinator has self destructed (cant test this yet)
    }

    function testSlashNode() public {
        testPostProcessDkg();

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);
        emit log_named_uint("The delegation reward of node1 before slash", node1DelegationRewardBefore);
        // slash node1
        uint256 pendingBlock = 0;

        controller.slashNodeForTest(node1, disqualifiedNodePenaltyAmount, pendingBlock);

        // Assert staking penalty applied to node1
        emit log_named_uint("The delegation reward of node1 after slash", staking.getDelegationReward(node1));
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
    }

    function testNodeQuit() public {
        // call nodeQuit with unregistered node: Fail
        vm.prank(node1);
        vm.expectRevert(abi.encodeWithSelector(Controller.NodeNotRegistered.selector));
        controller.nodeQuit();

        // register node, confirm initial stake amount
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);
        assertEq(1, controller.getGroup(0).members.length);
        // TODO
        // assertEq(operatorStakeAmount, staker.getLockedAmount(node1));
        printGroupInfo(0);
        // printNodeInfo(node1);

        // Quit node: Success
        vm.prank(node1);
        controller.nodeQuit();
        // assert member length is 0
        assertEq(0, controller.getGroup(0).members.length);
        // TODO
        // assertEq(0, staker.getLockedAmount(node1));
        // printGroupInfo(0);
        // printNodeInfo(node1);
    }
}
