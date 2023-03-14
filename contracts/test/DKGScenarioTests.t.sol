// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

pragma experimental ABIEncoderV2;

import {Coordinator} from "src/Coordinator.sol";
import {Controller} from "src/Controller.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";
import "src/interfaces/ICoordinator.sol";
import "./MockArpaEthOracle.sol";
import "./RandcastTestHelper.sol";

// Suggested usage: forge test --match-contract Controller -vv

contract DKGScenarioTest is RandcastTestHelper {
    uint256 nodeStakingAmount = 50000;
    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 10;
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;

    address public owner = address(0xC0FF33);

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
        MockArpaEthOracle oracle = new MockArpaEthOracle();
        controller = new Controller(address(arpa), address(oracle));

        controller.setControllerConfig(
            nodeStakingAmount,
            disqualifiedNodePenaltyAmount,
            defaultNumberOfCommitters,
            defaultDkgPhaseDuration,
            groupMaxCapacity,
            idealNumberOfGroups,
            pendingBlockAfterQuit,
            dkgPostProcessReward
        );

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
                Controller.CommitDkgParams(
                    params[i].groupIndex,
                    params[i].groupEpoch,
                    params[i].publicKey,
                    params[i].partialPublicKey,
                    params[i].disqualifiedNodes
                )
            );
        }
    }

    function testDkgReverts() public {
        // Test all "require" checks
        bytes memory err;
        Params[] memory params = new Params[](1);

        err = bytes("Group does not exist");
        params[0] = Params(node1, true, err, 999, 3, publicKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        err = bytes("Caller Group epoch does not match controller Group epoch");
        params[0] = Params(node1, true, err, 0, 999, publicKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        err = bytes("Node is not a member of the group");
        params[0] = Params(node6, true, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(BLS.InvalidPublicKeyEncoding.selector);
        params[0] = Params(node1, true, err, 0, 3, publicKey, hex"AAAA", new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(Adapter.InvalidPartialPublicKey.selector);
        params[0] = Params(node1, true, err, 0, 3, publicKey, badKey, new address[](0));
        dkgHelper(params);

        err = abi.encodeWithSelector(Adapter.InvalidPublicKey.selector);
        params[0] = Params(node1, true, err, 0, 3, badKey, partialPublicKey1, new address[](0));
        dkgHelper(params);

        // Happy Path
        Params[] memory params2 = new Params[](2);
        params2[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));

        err = bytes("CommitCache already contains PartialKey for this node");
        params2[1] = Params(node1, true, err, 0, 3, publicKey, partialPublicKey1, new address[](0));

        dkgHelper(params2);
    }

    function testDkgHappyPath() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, new address[](0));
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, new address[](0));
        dkgHelper(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);
        // printGroupInfo(0);
    }

    function testDkgSingleDisqualifiedNode() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](1);
        disqualifiedNodes[0] = node1;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, disqualifiedNodes);
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, disqualifiedNodes);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 4);
        assertEq(controller.getGroup(0).size, 4);

        // assert node1 was slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(nodeStakingAmount - disqualifiedNodePenaltyAmount, controller.getStakedAmount(node1));
        // printGroupInfo(0);
    }

    function testDkgTwoDisqualifiedNodes() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](2);
        disqualifiedNodes[0] = node1;
        disqualifiedNodes[1] = node2;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, disqualifiedNodes);
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 3);
        assertEq(controller.getGroup(0).size, 3);

        // assert node1 was slashed
        assertEq(nodeInGroup(node1, 0), false);
        assertEq(nodeInGroup(node2, 0), false);
        assertEq(nodeStakingAmount - disqualifiedNodePenaltyAmount, controller.getStakedAmount(node1));
        assertEq(nodeStakingAmount - disqualifiedNodePenaltyAmount, controller.getStakedAmount(node2));
        // printGroupInfo(0);
    }

    function testDkgThreeDisqualifiedNodes() public {
        Params[] memory params = new Params[](5);
        bytes memory err;
        address[] memory disqualifiedNodes = new address[](3);
        disqualifiedNodes[0] = node1;
        disqualifiedNodes[1] = node2;
        disqualifiedNodes[2] = node3;

        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, disqualifiedNodes);
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, disqualifiedNodes);
        dkgHelper(params);

        // * is this right? no issues.
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);
    }

    function testDkgAllDifferentDisqualifiedNodes() public {
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

        // assert group state is correct
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);

        // assert node1 was slashed
        assertEq(nodeStakingAmount, controller.getStakedAmount(node1));
        assertEq(nodeStakingAmount, controller.getStakedAmount(node2));
        assertEq(nodeStakingAmount, controller.getStakedAmount(node3));
        assertEq(nodeStakingAmount, controller.getStakedAmount(node4));
        assertEq(nodeStakingAmount, controller.getStakedAmount(node5));

        printGroupInfo(0);
    }
}
