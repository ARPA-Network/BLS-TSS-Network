// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

pragma experimental ABIEncoderV2;

import "./RandcastTestHelper.sol";
import {ICoordinator} from "../src/interfaces/ICoordinator.sol";
import {BLS} from "../src/libraries/BLS.sol";

contract DKGScenarioTest is RandcastTestHelper {
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

        // deal owner and create controller
        vm.deal(owner, 1 * 10 ** 18);

        vm.prank(owner);
        arpa = new ERC20("arpa token", "ARPA");

        address[] memory operators = new address[](5);
        operators[0] = node1;
        operators[1] = node2;
        operators[2] = node3;
        operators[3] = node4;
        operators[4] = node5;
        _prepareStakingContract(stakingDeployer, address(arpa), operators);

        controller = new ControllerForTest(address(arpa), lastOutput);

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

        // Register Nodes
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);
        vm.prank(node2);
        controller.nodeRegister(DKGPubkey2);
        vm.prank(node3);
        controller.nodeRegister(DKGPubkey3);
        vm.prank(node4);
        controller.nodeRegister(DKGPubkey4);
        vm.prank(node5);
        controller.nodeRegister(DKGPubkey5);
    }

    struct Params {
        address nodeIdAddress;
        bool shouldRevert;
        bytes revertMessage;
        uint256 groupIndex;
        uint256 groupEpoch;
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
    }

    function dkgHelper(Params[] memory params) public {
        for (uint256 i = 0; i < params.length; i++) {
            vm.prank(params[i].nodeIdAddress);
            if (params[i].shouldRevert) {
                vm.expectRevert(params[i].revertMessage);
            }
            controller.commitDkg(
                IController.CommitDkgParams(
                    params[i].groupIndex,
                    params[i].groupEpoch,
                    params[i].publicKey,
                    params[i].partialPublicKey,
                    params[i].disqualifiedNodes
                )
            );
        }
    }

    function setPhase(uint256 groupIndex, uint256 phase) public {
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();
        vm.roll(startBlock + 1 + phase * defaultDkgPhaseDuration);
    }

    function testDkgReverts() public {
        // Test all "require" checks
        bytes memory err;
        Params[] memory params = new Params[](1);

        err = abi.encodeWithSelector(Controller.GroupNotExist.selector, 999);
        params[0] = Params(node1, true, err, 999, 3, publicKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(Controller.EpochMismatch.selector, 0, 999, 3);
        params[0] = Params(node1, true, err, 0, 999, publicKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(Controller.NodeNotInGroup.selector, 0, address(node6));
        params[0] = Params(node6, true, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(BLS.InvalidPublicKeyEncoding.selector);
        params[0] = Params(node1, true, err, 0, 3, publicKey, hex"AAAA", new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(BLS.InvalidPartialPublicKey.selector);
        params[0] = Params(node1, true, err, 0, 3, publicKey, badKey, new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(BLS.InvalidPublicKey.selector);
        params[0] = Params(node1, true, err, 0, 3, badKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        // Happy Path
        Params[] memory params2 = new Params[](2);
        params2[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));

        err = abi.encodeWithSelector(Controller.PartialKeyAlreadyRegistered.selector, 0, address(node1));
        params2[1] = Params(node1, true, err, 0, 3, publicKey, partialPublicKey1, new address[](0));

        dkgHelper(params2);
    }

    // * Happy Path
    function testDkgHappyPath() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, new address[](0));
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, new address[](0));
        dkgHelper(params);
        // printGroupInfo(0);

        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);
    }

    // * 1 Disqualified Node
    function test1Dq4Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](1);
        disqualifiedNodes[0] = node1;

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, disqualifiedNodes);
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, disqualifiedNodes);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 4);
        assertEq(controller.getGroup(0).size, 4);

        // assert node1 was slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
    }

    function test1Dq3Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](1);
        disqualifiedNodes[0] = node1;

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, disqualifiedNodes);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 4);
        assertEq(controller.getGroup(0).size, 4);

        // assert node1 was slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
    }

    function test1Dq2Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](1);
        disqualifiedNodes[0] = node1;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);
    }

    function test1Dq1Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](1);
        disqualifiedNodes[0] = node1;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, new address[](0));
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);
    }

    // * 2 Disqualified Nodes

    function test2Dq4Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](2);
        disqualifiedNodes[0] = node1;
        disqualifiedNodes[1] = node2;

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);
        uint256 node2DelegationRewardBefore = staking.getDelegationReward(node2);

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, disqualifiedNodes);
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, disqualifiedNodes);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 3);
        assertEq(controller.getGroup(0).size, 3);

        // assert node1 was slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
        // assert node2 was slashed
        assertEq(nodeInGroup(node2, 0), false);
        assertEq(node2DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node2));
    }

    function test2Dq3Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](2);
        disqualifiedNodes[0] = node1;
        disqualifiedNodes[1] = node2;

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);
        uint256 node2DelegationRewardBefore = staking.getDelegationReward(node2);

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, disqualifiedNodes);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 3);
        assertEq(controller.getGroup(0).size, 3);

        // assert node1 and node2 were slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
        assertEq(nodeInGroup(node2, 0), false);
        assertEq(node2DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node2));
    }

    function test2Dq2Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](2);
        disqualifiedNodes[0] = node1;
        disqualifiedNodes[1] = node2;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);
    }

    // * 3 Disqualified Nodes (???)
    function test3Dq3Reporter() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](3);
        disqualifiedNodes[0] = node1;
        disqualifiedNodes[1] = node2;
        disqualifiedNodes[2] = node3;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, disqualifiedNodes);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);
        // printGroupInfo(0);

        // * Is this okay?
        // * When non-disqualified majority members < g.threshold (3), group is not formed, nodes are not slashed.

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);

        // assert node1, node2, and node3 were slashed
        assertEq(nodeInGroup(node1, 0), true);
        // assertEq(nodeStakingAmount, staking.getDelegationReward(node1));
        // assertEq(nodeInGroup(node1, 0), false);
        // // assertEq(nodeStakingAmount - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
    }

    // * PPDKG with 3 Disqualified Nodes
    function testPPDKG3Dq3Reporter() public {
        test3Dq3Reporter();
        // printGroupInfo(0);

        // Set the coordinator to completed phase
        setPhase(0, 4);

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);
        uint256 node2DelegationRewardBefore = staking.getDelegationReward(node2);
        uint256 node3DelegationRewardBefore = staking.getDelegationReward(node3);

        // call postProcessDkg as node1
        vm.prank(node1);
        controller.postProcessDkg(0, 3);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(controller.getGroup(0).members.length, 2);
        assertEq(controller.getGroup(0).size, 2);

        // assert node1, node2, and node3 were slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
        assertEq(nodeInGroup(node2, 0), false);
        assertEq(node2DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node2));
        assertEq(nodeInGroup(node3, 0), false);
        assertEq(node3DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node3));
    }

    // *  Disqualified Node Mixed Reporting
    function testMixed1Dq5Reporter5Target() public {
        // 5 nodes all report different disqualified nodes (1 reports 2 reports 3 ...)
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory dn1 = new address[](1);
        address[] memory dn2 = new address[](1);
        address[] memory dn3 = new address[](1);
        address[] memory dn4 = new address[](1);
        address[] memory dn5 = new address[](1);

        dn1[0] = node1;
        dn2[0] = node2;
        dn3[0] = node3;
        dn4[0] = node4;
        dn5[0] = node5;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, dn2);
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, dn3);
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, dn4);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, dn5);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, dn1);
        dkgHelper(params);
        // printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);
    }

    // * PPDKG Node Mixed Reporting
    function testPPDKGMixed1Dq5Reporter5Target() public {
        testMixed1Dq5Reporter5Target();
        printGroupInfo(0);

        // Set the coordinator to completed phase
        setPhase(0, 4);

        uint256 node1DelegationRewardBefore = staking.getDelegationReward(node1);
        uint256 node2DelegationRewardBefore = staking.getDelegationReward(node2);
        uint256 node3DelegationRewardBefore = staking.getDelegationReward(node3);
        uint256 node4DelegationRewardBefore = staking.getDelegationReward(node4);
        uint256 node5DelegationRewardBefore = staking.getDelegationReward(node5);

        // call postProcessDkg as node1
        vm.prank(node1);
        controller.postProcessDkg(0, 3);
        printGroupInfo(0);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(controller.getGroup(0).members.length, 0);
        assertEq(controller.getGroup(0).size, 0);

        // assert nodes are slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(node1DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node1));
        assertEq(nodeInGroup(node2, 0), false);
        assertEq(node2DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node2));
        assertEq(nodeInGroup(node3, 0), false);
        assertEq(node3DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node3));
        assertEq(nodeInGroup(node4, 0), false);
        assertEq(node4DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node4));
        assertEq(nodeInGroup(node5, 0), false);
        assertEq(node5DelegationRewardBefore - disqualifiedNodePenaltyAmount, staking.getDelegationReward(node5));
    }
}
