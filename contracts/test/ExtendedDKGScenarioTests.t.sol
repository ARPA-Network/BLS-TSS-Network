// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

pragma experimental ABIEncoderV2;

import {Coordinator} from "src/Coordinator.sol";
import {Controller} from "src/Controller.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";
import "src/interfaces/ICoordinator.sol";
import "./MockArpaEthOracle.sol";
import "./RandcastTestHelper.sol";

contract ExtendedDKGScenarioTest is RandcastTestHelper {
    uint256 nodeStakingAmount = 50000;
    uint256 disqualifiedNodePenaltyAmount = 1000;
    uint256 defaultNumberOfCommitters = 3;
    uint256 defaultDkgPhaseDuration = 10;
    uint256 groupMaxCapacity = 6; // ! Set to 6 for extended rebalancing tests.
    uint256 idealNumberOfGroups = 5;
    uint256 pendingBlockAfterQuit = 100;
    uint256 dkgPostProcessReward = 100;

    address public owner = address(0xC0FF33);

    // * Setup

    function setUp() public {
        // add 31 test nodes
        addTestNode(1, node1, DKGPubkey1, partialPublicKey1);
        addTestNode(2, node2, DKGPubkey2, partialPublicKey2);
        addTestNode(3, node3, DKGPubkey3, partialPublicKey3);
        addTestNode(4, node4, DKGPubkey4, partialPublicKey4);
        addTestNode(5, node5, DKGPubkey5, partialPublicKey5);
        addTestNode(6, node6, DKGPubkey6, partialPublicKey6);
        addTestNode(7, node7, DKGPubkey7, partialPublicKey7);
        addTestNode(8, node8, DKGPubkey8, partialPublicKey8);
        addTestNode(9, node9, DKGPubkey9, partialPublicKey9);
        addTestNode(10, node10, DKGPubkey10, partialPublicKey10);
        addTestNode(11, node11, DKGPubkey11, partialPublicKey11);
        addTestNode(12, node12, DKGPubkey12, partialPublicKey12);
        addTestNode(13, node13, DKGPubkey13, partialPublicKey13);
        addTestNode(14, node14, DKGPubkey14, partialPublicKey14);
        addTestNode(15, node15, DKGPubkey15, partialPublicKey15);
        addTestNode(16, node16, DKGPubkey16, partialPublicKey16);
        addTestNode(17, node17, DKGPubkey17, partialPublicKey17);
        addTestNode(18, node18, DKGPubkey18, partialPublicKey18);
        addTestNode(19, node19, DKGPubkey19, partialPublicKey19);
        addTestNode(20, node20, DKGPubkey20, partialPublicKey20);
        addTestNode(21, node21, DKGPubkey21, partialPublicKey21);
        addTestNode(22, node22, DKGPubkey22, partialPublicKey22);
        addTestNode(23, node23, DKGPubkey23, partialPublicKey23);
        addTestNode(24, node24, DKGPubkey24, partialPublicKey24);
        addTestNode(25, node25, DKGPubkey25, partialPublicKey25);
        addTestNode(26, node26, DKGPubkey26, partialPublicKey26);
        addTestNode(27, node27, DKGPubkey27, partialPublicKey27);
        addTestNode(28, node28, DKGPubkey28, partialPublicKey28);
        addTestNode(29, node29, DKGPubkey29, partialPublicKey29);
        addTestNode(30, node30, DKGPubkey30, partialPublicKey30);
        addTestNode(31, node31, DKGPubkey31, partialPublicKey31);

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

    // * Test Node Setup
    function testNodeSetup() public {
        bytes memory dkgPublicKey;
        address nodeIdAddress;
        uint256[4] memory publicKey;
        uint256[4] memory partialPublicKey;

        for (uint256 i = 1; i <= 31; i++) {
            nodeIdAddress = testNodes[i].nodeAddress;
            dkgPublicKey = testNodes[i].publicKey;
            publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
            partialPublicKey = BLS.fromBytesPublicKey(testNodes[i].partialPublicKey);
            assertEq(BLS.isValidPublicKey(publicKey), true);
            assertEq(BLS.isValidPublicKey(partialPublicKey), true);
        }
    }

    // * Node Register Helper  Testing
    mapping(uint256 => TestNode) testNodes;

    struct TestNode {
        address nodeAddress;
        bytes publicKey;
        bytes partialPublicKey;
    }

    // Add a test node to the testNodes mapping and deal eth: Used for setup
    function addTestNode(uint256 index, address nodeAddress, bytes memory publicKey, bytes memory partialPublicKey)
        public
    {
        TestNode memory newNode =
            TestNode({nodeAddress: nodeAddress, publicKey: publicKey, partialPublicKey: partialPublicKey});

        testNodes[index] = newNode;
        vm.deal(nodeAddress, 1 * 10 ** 18);
    }

    // Take in a uint256 specifying node index, call node register using info from testNodes mapping
    function registerIndex(uint256 nodeIndex) public {
        vm.prank(testNodes[nodeIndex].nodeAddress);
        controller.nodeRegister(testNodes[nodeIndex].publicKey);
    }

    // * Commit DKG Helper Functions

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

    // * ////////////////////////////////////////////////////////////////////////////////
    // * Extended Scenario Tests Begin (Rebalancing, Regrouping, Various Edgecases etc..)
    // * ////////////////////////////////////////////////////////////////////////////////

    // * Regroup remaining nodes after nodeQuit: (5 -> 4)
    function test5NodeQuit() public {
        // register nodes 1-5 using registerHelper()
        assertEq(controller.getGroup(0).epoch, 0);
        registerIndex(1);
        registerIndex(2);
        registerIndex(3); // controller emits event here
        assertEq(controller.getGroup(0).epoch, 1); // g.epoch++
        registerIndex(4); // here
        assertEq(controller.getGroup(0).epoch, 2); // g.epoch++
        registerIndex(5); // and here
        assertEq(controller.getGroup(0).epoch, 3); // g.epoch++

        // group the 5 nodes using commitdkg.
        Params[] memory params = new Params[](5);
        bytes memory err;
        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, new address[](0));
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, new address[](0));
        dkgHelper(params);

        // assert group info
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);

        // node 1 calls nodeQuit
        vm.prank(node1);
        controller.nodeQuit(); // controller emits event to start dkg proccess
        assertEq(controller.getGroup(0).epoch, 4); // g.epoch++

        // node 2-4 call commitdkg
        params = new Params[](4);
        params[0] = Params(node2, false, err, 0, 4, publicKey, partialPublicKey2, new address[](0));
        params[1] = Params(node3, false, err, 0, 4, publicKey, partialPublicKey3, new address[](0));
        params[2] = Params(node4, false, err, 0, 4, publicKey, partialPublicKey4, new address[](0));
        params[3] = Params(node5, false, err, 0, 4, publicKey, partialPublicKey5, new address[](0));
        dkgHelper(params);

        // check group info
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 4);
        assertEq(controller.getGroup(0).size, 4);
        printGroupInfo(0);
    }

    // //* Rebalance two groups after nodeQuit results in group falling below threshold (5,3) -> (3,4)
    function test53NodeQuit() public {
        // * Register and group 5 nodes to group_0
        assertEq(controller.getGroup(0).epoch, 0);
        registerIndex(1);
        registerIndex(2);
        registerIndex(3); // controller emits event here (1-3 call commitDkg)
        assertEq(controller.getGroup(0).epoch, 1); // g.epoch++
        registerIndex(4); // here (1-4 call commitDkg)
        assertEq(controller.getGroup(0).epoch, 2); // g.epoch++
        registerIndex(5); // here
        assertEq(controller.getGroup(0).epoch, 3); // g.epoch++

        // group the 5 nodes using commitdkg.
        Params[] memory params = new Params[](5);
        bytes memory err;
        params[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));
        params[1] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params[2] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, new address[](0));
        params[4] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, new address[](0));
        dkgHelper(params);

        // assert group info
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(controller.getGroup(0).members.length, 5);
        assertEq(controller.getGroup(0).size, 5);

        // * Register and group 5 new nodes
        assertEq(controller.getGroup(0).epoch, 3); // initial state
        assertEq(controller.getGroup(1).epoch, 0); // initial state

        registerIndex(6); // Groups are rebalanced to (3,3) group_0 and group_1 epoch's are incremented here.
        assertEq(controller.getGroup(0).epoch, 4); // g.epoch++
        assertEq(controller.getGroup(1).epoch, 1); // g.epoch++
        assertEq(controller.getGroup(0).size, 3);
        assertEq(controller.getGroup(1).size, 3);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(7); // added to group_0, only group_0 epoch is incremented
        assertEq(controller.getGroup(0).epoch, 5); // g.epoch++
        assertEq(controller.getGroup(1).epoch, 1); // no change
        assertEq(controller.getGroup(0).size, 4); // g.size++
        assertEq(controller.getGroup(1).size, 3);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(8); // added to group_1, only group_1 epoch is incremented
        assertEq(controller.getGroup(0).epoch, 5); // no change
        assertEq(controller.getGroup(1).epoch, 2); // g.epoch++
        assertEq(controller.getGroup(0).size, 4);
        assertEq(controller.getGroup(1).size, 4); // g.size++
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(9); // added to group_0, only group_0 epoch is incremented
        assertEq(controller.getGroup(0).epoch, 6); // g.epoch++
        assertEq(controller.getGroup(1).epoch, 2); // no change
        assertEq(controller.getGroup(0).size, 5); // g.size++
        assertEq(controller.getGroup(1).size, 4);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(10); // added to group_1, only group_1 epoch is incremented
        assertEq(controller.getGroup(0).epoch, 6); // no change
        assertEq(controller.getGroup(1).epoch, 3); // g.epoch++
        assertEq(controller.getGroup(0).size, 5);
        assertEq(controller.getGroup(1).size, 5); // g.size++
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);
        // groups have been reshuffled, current indexes are as follows:
        // group_0 (5,4,3,7,9), group_1 (6,1,2,8,10)

        // * Remove two nodes from group_1 (node8, node10) so that group_1 size == 3
        vm.prank(node8);
        controller.nodeQuit(); // group_1 epoch is incremented here
        assertEq(controller.getGroup(1).epoch, 4); // g.epoch++
        assertEq(controller.getGroup(1).size, 4); // g.size--

        vm.prank(node10);
        controller.nodeQuit(); // group_1 epoch is incremented here
        assertEq(controller.getGroup(1).epoch, 5); // g.epoch++
        assertEq(controller.getGroup(1).size, 3); // g.size--

        // * (5,3) configuration reached: group_0 (5,4,3,7,9) / group_1 (6,1,2)
        assertEq(controller.getGroup(0).size, 5);
        assertEq(controller.getGroup(1).size, 3);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);
        assertEq(controller.getGroup(0).epoch, 6);
        assertEq(controller.getGroup(1).epoch, 5);

        // * group group_0 and group_1 with commitDKG
        params = new Params[](5);
        params[0] = Params(node5, false, err, 0, 6, publicKey, partialPublicKey5, new address[](0));
        params[1] = Params(node4, false, err, 0, 6, publicKey, partialPublicKey4, new address[](0));
        params[2] = Params(node3, false, err, 0, 6, publicKey, partialPublicKey3, new address[](0));
        params[3] = Params(node7, false, err, 0, 6, publicKey, partialPublicKey7, new address[](0));
        params[4] = Params(node9, false, err, 0, 6, publicKey, partialPublicKey9, new address[](0));
        dkgHelper(params);

        params = new Params[](3);
        params[0] = Params(node6, false, err, 1, 5, publicKey, partialPublicKey6, new address[](0));
        params[1] = Params(node1, false, err, 1, 5, publicKey, partialPublicKey1, new address[](0));
        params[2] = Params(node2, false, err, 1, 5, publicKey, partialPublicKey2, new address[](0));
        dkgHelper(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), true);
        assertEq(controller.getGroup(0).epoch, 6); // g.epoch++
        assertEq(controller.getGroup(1).epoch, 5); // g.epoch++

        // * node in group_1 quits (node6)
        vm.prank(node6);
        controller.nodeQuit();
        // group_1 falls below threshold, rebalancing occurs to (3,4), event emitted for both groups
        assertEq(controller.getGroup(0).epoch, 7); // g.epoch++
        assertEq(controller.getGroup(1).epoch, 6); // g.epoch++
        assertEq(controller.getGroup(0).size, 3);
        assertEq(controller.getGroup(1).size, 4);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        // * group group_0 (9,7,3) and group_1 (2,1,5,4) with commitDKG
        params = new Params[](3);
        params[0] = Params(node9, false, err, 0, 7, publicKey, partialPublicKey9, new address[](0));
        params[1] = Params(node7, false, err, 0, 7, publicKey, partialPublicKey7, new address[](0));
        params[2] = Params(node3, false, err, 0, 7, publicKey, partialPublicKey3, new address[](0));
        dkgHelper(params);

        params = new Params[](4);
        params[0] = Params(node2, false, err, 1, 6, publicKey, partialPublicKey2, new address[](0));
        params[1] = Params(node1, false, err, 1, 6, publicKey, partialPublicKey1, new address[](0));
        params[2] = Params(node5, false, err, 1, 6, publicKey, partialPublicKey5, new address[](0));
        params[3] = Params(node4, false, err, 1, 6, publicKey, partialPublicKey4, new address[](0));
        dkgHelper(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), true);

        printGroupInfo(0);
        printGroupInfo(1);
    }

    // * For the following tests we focus on Rebalancing logic rather than CommitDKG() details
    function test66NodeRegister() public {
        // Setup group_0 and group_1 so that they have 6 grouped nodes each
        registerIndex(1);
        registerIndex(2);
        registerIndex(3);
        registerIndex(4);
        registerIndex(5);
        registerIndex(6);
        registerIndex(7);
        registerIndex(8);
        registerIndex(9);
        registerIndex(10);
        registerIndex(11);
        registerIndex(12);

        // group up group0 (6,4,5,8,9,11)
        bytes memory err;
        Params[] memory params = new Params[](6);
        params[0] = Params(node6, false, err, 0, 8, publicKey, DKGPubkey6, new address[](0));
        params[1] = Params(node4, false, err, 0, 8, publicKey, DKGPubkey4, new address[](0));
        params[2] = Params(node5, false, err, 0, 8, publicKey, DKGPubkey5, new address[](0));
        params[3] = Params(node8, false, err, 0, 8, publicKey, DKGPubkey8, new address[](0));
        params[4] = Params(node9, false, err, 0, 8, publicKey, DKGPubkey9, new address[](0));
        params[5] = Params(node11, false, err, 0, 8, publicKey, DKGPubkey11, new address[](0));
        dkgHelper(params);

        // group up group1 (1,2,3,7,10,12)
        params = new Params[](6);
        params[0] = Params(node1, false, err, 1, 3, publicKey, DKGPubkey1, new address[](0));
        params[1] = Params(node2, false, err, 1, 3, publicKey, DKGPubkey2, new address[](0));
        params[2] = Params(node3, false, err, 1, 3, publicKey, DKGPubkey3, new address[](0));
        params[3] = Params(node7, false, err, 1, 3, publicKey, DKGPubkey7, new address[](0));
        params[4] = Params(node10, false, err, 1, 3, publicKey, DKGPubkey10, new address[](0));
        params[5] = Params(node12, false, err, 1, 3, publicKey, DKGPubkey12, new address[](0));
        dkgHelper(params);

        // printGroupInfo(0);
        // printGroupInfo(1);

        // New node calls node register.
        registerIndex(13);

        printGroupInfo(0);
        printGroupInfo(1);
        printGroupInfo(2);

        // ! Rebalanced to 3,4,6, but only group_1 functional.

        // Todo: Make a group info printer that prints all groups that contain nodes, showing which nodes are in each group and their group status.
    }

    function test33NodeRegister() public {
        // Setup group_0 and group_1 so that they have 6 grouped nodes each
        registerIndex(1);
        registerIndex(2);
        registerIndex(3);
        // * commit dkg with 1-3 (group_0)

        registerIndex(4);
        // * group_1
        registerIndex(5);
        registerIndex(6);
        // * commit dkg here.

        // ! how do I get the nodes into a (3,3) configuration???
    }
}
