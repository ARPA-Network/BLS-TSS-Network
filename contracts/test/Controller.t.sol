// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

pragma experimental ABIEncoderV2;

import {Coordinator} from "src/Coordinator.sol";
import "src/interfaces/ICoordinator.sol";
import "./RandcastTestHelper.sol";

// Suggested usage: forge test --match-contract ControllerTest -vv

contract ControllerTest is RandcastTestHelper {
    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 10;
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;

    address public owner = admin;

    function setUp() public {
        // deal nodes
        vm.deal(node1, 1 * 10 ** 18);
        vm.deal(node2, 1 * 10 ** 18);
        vm.deal(node3, 1 * 10 ** 18);
        vm.deal(node4, 1 * 10 ** 18);
        vm.deal(node5, 1 * 10 ** 18);

        // deal owner and create controller
        vm.deal(owner, 1 * 10 ** 18);
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
        prepareStakingContract(stakingDeployer, address(arpa), operators);

        vm.prank(owner);
        MockArpaEthOracle oracle = new MockArpaEthOracle();

        vm.prank(owner);
        controller = new ControllerForTest(address(arpa), address(oracle));

        vm.prank(owner);
        controller.setControllerConfig(
            address(staking),
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
        // printNodeInfo(node1);
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);
        // printNodeInfo(node1);

        Controller.Node memory n = controller.getNode(node1);
        assertEq(n.idAddress, node1);
        assertEq(n.dkgPublicKey, DKGPubkey1);
        assertEq(n.state, true);
        assertEq(n.pendingUntilBlock, 0);

        vm.expectRevert("Node is already registered");
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);
    }

    function testRemoveFromGroup() public {
        testCommitDkg();
        printGroupInfo(0);
        assertEq(controller.getGroup(0).size, 3);
        controller.removeFromGroupForTest(address(0x1), 0, false);
        printGroupInfo(0);
        assertEq(controller.getGroup(0).size, 2);
    }

    function testRebalanceGroup() public {
        emit log_named_uint("groupCount", controller.groupCount());
        testCommitDkg();
        emit log_named_uint("groupCount", controller.groupCount());
        printGroupInfo(0);

        // Add 4th node, should create new group
        vm.prank(node4);
        controller.nodeRegister(DKGPubkey4);
        emit log_named_uint("groupCount", controller.groupCount());
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
        printGroupInfo(0);
        printGroupInfo(1);
        emit log("++++++ Rebalance 2 +++++++");
        output = controller.rebalanceGroupForTest(0, 1);
        assertEq(output, true);
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
        // * fail emit group event if group does not exist
        vm.expectRevert("Group does not exist");
        controller.emitGroupEventForTest(99999);

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
        Controller.Group memory g = controller.getGroup(groupIndex);
        assertEq(g.index, 0);
        assertEq(g.epoch, 1);
        assertEq(g.size, 3);
        assertEq(g.threshold, 3);
        assertEq(g.members.length, 3);

        // Verify node2 info is recorded in group.members[1]
        Controller.Member memory m = g.members[1];
        // printMemberInfo(groupIndex, 1);
        assertEq(m.nodeIdAddress, node2);
        // assertEq(m.partialPublicKey, TODO);

        // address coordinatorAddress = controller.getCoordinator(groupIndex);
        // emit log_named_address("\nCoordinator", coordinatorAddress);
    }

    function testValidGroupIndices() public {
        uint256[] memory groupIndices = controller.validGroupIndices();
        assertEq(groupIndices.length, 0);
        assertEq(controller.groupCount(), 0);

        testCommitDkg();

        groupIndices = controller.validGroupIndices();
        // for (uint256 i = 0; i < groupIndices.length; i++) {
        //     emit log_named_uint("groupIndices[i]", groupIndices[i]);
        // }
        assertEq(groupIndices.length, 1);
        assertEq(controller.groupCount(), 1);
    }

    function testFindOrCreateTargetGroup() public {
        emit log_named_uint("groupCount", controller.groupCount());
        testCommitDkg();
        emit log_named_uint("groupCount", controller.groupCount());
        printGroupInfo(1);

        // Add 4th node, should create new group
        vm.prank(node4);
        controller.nodeRegister(DKGPubkey4);
        emit log_named_uint("groupCount", controller.groupCount());
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
        vm.expectRevert("Group does not exist");
        Controller.CommitDkgParams memory params = Controller.CommitDkgParams(
            999, // wrong group index
            groupEpoch,
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        controller.commitDkg(params);

        // Fail if group does not match Controller Group Epoch
        vm.prank(node1);
        vm.expectRevert("Caller Group epoch does not match controller Group epoch");
        params = Controller.CommitDkgParams(
            groupIndex,
            999, //  wrong epoch
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        controller.commitDkg(params);

        // Fail if node is not a member of the group
        vm.prank(node5);
        vm.expectRevert("Node is not a member of the group");
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        controller.commitDkg(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        // Succesful Commit: Node 1
        vm.prank(node1);
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex);

        //  Fail if CommitCache already contains PartialKey for this node
        vm.prank(node1);
        vm.expectRevert("CommitCache already contains PartialKey for this node");
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey, disqualifiedNodes);
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);

        // Succesful Commit: Node 2
        vm.prank(node2);
        params = Controller.CommitDkgParams(
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
        params = Controller.CommitDkgParams(
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
        uint64 lastOutput = 0x2222222222222222;

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
        assertEq(controller.partialKeyRegistered(groupIndex, node1), false);
    }

    function testPostProcessDkg() public {
        testCommitDkg();

        uint256 groupIndex = 0;
        uint256 groupEpoch = 1;
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();

        vm.expectRevert("Group does not exist");
        controller.postProcessDkg(99999, 0); //(groupIndex, groupEpoch))

        vm.prank(node12);
        vm.expectRevert("Node is not a member of the group");
        controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(node1);
        vm.expectRevert("Caller Group epoch does not match Controller Group epoch");
        controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(node1);
        vm.expectRevert("DKG still in progress");
        controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))

        // Set the coordinator to completed phase
        vm.roll(startBlock + 1 + 4 * defaultDkgPhaseDuration); // Put the coordinator in phase

        // Succesful post process dkg: HAPPY PATH
        vm.startPrank(node1);
        controller.postProcessDkg(groupIndex, groupEpoch);
        uint256 nodeRewards = controller.getRewards(node1);
        emit log_named_uint("node1 rewards", nodeRewards);
        assertEq(nodeRewards, dkgPostProcessReward);

        // test self destruct worked properly
        address emptyCoordinatorAddress = controller.getCoordinator(groupIndex);
        assertEq(emptyCoordinatorAddress, address(0));

        vm.expectRevert("Coordinator not found for groupIndex");
        controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))
        vm.stopPrank();
        // assert that coordinator has self destructed (cant test this yet)
    }

    function testSlashNode() public {
        testPostProcessDkg();
        Controller.Group memory g = controller.getGroup(0);
        assertEq(g.members.length, 3);
        assertEq(g.isStrictlyMajorityConsensusReached, true);
        printGroupInfo(0);

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);
        emit log_named_uint("The delegation reward of node1 before slash", node1DelegationRewardBefore);
        // slash node1
        uint256 pendingBlock = 0;
        bool handleGroup = true;

        controller.slashNodeForTest(node1, disqualifiedNodePenaltyAmount, pendingBlock, handleGroup);

        // Assert staking penalty applied to node1
        emit log_named_uint("The delegation reward of node1 after slash", staking.getDelegationReward(node1));
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));

        // assert node1 has been removed from the group
        g = controller.getGroup(0);
        assertEq(g.members.length, 2);
        assertEq(g.isStrictlyMajorityConsensusReached, false);

        printGroupInfo(0);
    }

    function testNodeQuit() public {
        // call nodeQuit with unregistered node: Fail
        vm.prank(node1);
        vm.expectRevert("Node not registered.");
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
