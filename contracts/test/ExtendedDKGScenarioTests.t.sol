// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

pragma experimental ABIEncoderV2;

import {Coordinator} from "src/Coordinator.sol";
import {Controller} from "src/Controller.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";
import "src/interfaces/ICoordinator.sol";
import "./MockArpaEthOracle.sol";
import "./RandcastTestHelper.sol";

contract DKGScenarioTest is RandcastTestHelper {
    uint256 nodeStakingAmount = 50000;
    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 6; // ! Set to 6 for extended rebalancing tests.
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;

    address public owner = address(0xC0FF33);

    // * Commit DKG Testing

    function setPhase(uint256 groupIndex, uint256 phase) public {
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();
        vm.roll(startBlock + 1 + phase * defaultDkgPhaseDuration);
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

    // * Setup

    function setUp() public {
        // add 31 test nodes
        addTestNode(1, node1, DKGPubkey1);
        addTestNode(2, node2, DKGPubkey2);
        addTestNode(3, node3, DKGPubkey3);
        addTestNode(4, node4, DKGPubkey4);
        addTestNode(5, node5, DKGPubkey5);
        addTestNode(6, node6, DKGPubkey6);
        addTestNode(7, node7, DKGPubkey7);
        addTestNode(8, node8, DKGPubkey8);
        addTestNode(9, node9, DKGPubkey9);
        addTestNode(10, node10, DKGPubkey10);
        addTestNode(11, node11, DKGPubkey11);
        addTestNode(12, node12, DKGPubkey12);
        addTestNode(13, node13, DKGPubkey13);
        addTestNode(14, node14, DKGPubkey14);
        addTestNode(15, node15, DKGPubkey15);
        addTestNode(16, node16, DKGPubkey16);
        addTestNode(17, node17, DKGPubkey17);
        addTestNode(18, node18, DKGPubkey18);
        addTestNode(19, node19, DKGPubkey19);
        addTestNode(20, node20, DKGPubkey20);
        addTestNode(21, node21, DKGPubkey21);
        addTestNode(22, node22, DKGPubkey22);
        addTestNode(23, node23, DKGPubkey23);
        addTestNode(24, node24, DKGPubkey24);
        addTestNode(25, node25, DKGPubkey25);
        addTestNode(26, node26, DKGPubkey26);
        addTestNode(27, node27, DKGPubkey27);
        addTestNode(28, node28, DKGPubkey28);
        addTestNode(29, node29, DKGPubkey29);
        addTestNode(30, node30, DKGPubkey30);
        addTestNode(31, node31, DKGPubkey31);

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
    }

    function testNodeSetup() public {
        bytes memory dkgPublicKey;
        address nodeIdAddress;
        uint256[4] memory publicKey;

        for (uint256 i = 1; i <= 31; i++) {
            nodeIdAddress = testNodes[i].nodeAddress;
            dkgPublicKey = testNodes[i].publicKey;
            publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
            assertEq(BLS.isValidPublicKey(publicKey), true);
        }
    }

    // * Node Register Testing
    mapping(uint256 => TestNode) testNodes;

    struct TestNode {
        address nodeAddress;
        bytes publicKey;
    }

    function addTestNode(uint256 index, address nodeAddress, bytes memory publicKey) public {
        TestNode memory newNode = TestNode({nodeAddress: nodeAddress, publicKey: publicKey});

        testNodes[index] = newNode;
        vm.deal(nodeAddress, 1 * 10 ** 18);
    }
    // Take in a list of uint256 specifying node indexes, call node register using info from testNodes mapping

    function registerHelper(uint256[] memory nodeIndexes) public {
        for (uint256 i = 0; i < nodeIndexes.length; i++) {
            vm.prank(testNodes[nodeIndexes[i]].nodeAddress);
            controller.nodeRegister(testNodes[nodeIndexes[i]].publicKey);
        }
    }

    // * Happy Path
    function testRegroupSingle() public {
        // register nodes 1-5 using registerHelper()
        uint256[] memory nodeIndexes = new uint256[](5);
        nodeIndexes[0] = 1;
        nodeIndexes[1] = 2;
        nodeIndexes[2] = 3;
        nodeIndexes[3] = 4;
        nodeIndexes[4] = 5;
        registerHelper(nodeIndexes);

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

        printGroupInfo(0);
        vm.prank(node1);
        controller.nodeQuit();
        printGroupInfo(0);
    }
}
