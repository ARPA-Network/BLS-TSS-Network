// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

pragma experimental ABIEncoderV2;

import {RandcastTestHelper, ERC20, ControllerForTest, IController} from "./RandcastTestHelper.sol";
import {ICoordinator} from "../src/interfaces/ICoordinator.sol";
import {BLS} from "../src/libraries/BLS.sol";

contract ExtendedDKGScenarioTest is RandcastTestHelper {
    uint256 internal _nodeStakingAmount = 50000;
    uint256 internal _disqualifiedNodePenaltyAmount = 1000;
    uint256 internal _defaultNumberOfCommitters = 3;
    uint256 internal _defaultDkgPhaseDuration = 10;
    uint256 internal _groupMaxCapacity = 6; // ! Set to 6 for extended rebalancing tests.
    uint256 internal _idealNumberOfGroups = 5;
    uint256 internal _pendingBlockAfterQuit = 100;
    uint256 internal _dkgPostProcessReward = 100;
    uint64 internal _lastOutput = 0x2222222222222222;

    address internal _owner = _admin;

    // * Setup

    function setUp() public {
        // add 31 test nodes
        addTestNode(1, _node1, _dkgPubkey1, _partialPublicKey1);
        addTestNode(2, _node2, _dkgPubkey2, _partialPublicKey2);
        addTestNode(3, _node3, _dkgPubkey3, _partialPublicKey3);
        addTestNode(4, _node4, _dkgPubkey4, _partialPublicKey4);
        addTestNode(5, _node5, _dkgPubkey5, _partialPublicKey5);
        addTestNode(6, _node6, _dkgPubkey6, _partialPublicKey6);
        addTestNode(7, _node7, _dkgPubkey7, _partialPublicKey7);
        addTestNode(8, _node8, _dkgPubkey8, _partialPublicKey8);
        addTestNode(9, _node9, _dkgPubkey9, _partialPublicKey9);
        addTestNode(10, _node10, _dkgPubkey10, _partialPublicKey10);
        addTestNode(11, _node11, _dkgPubkey11, _partialPublicKey11);
        addTestNode(12, _node12, _dkgPubkey12, _partialPublicKey12);
        addTestNode(13, _node13, _dkgPubkey13, _partialPublicKey13);
        addTestNode(14, _node14, _dkgPubkey14, _partialPublicKey14);
        addTestNode(15, _node15, _dkgPubkey15, _partialPublicKey15);
        addTestNode(16, _node16, _dkgPubkey16, _partialPublicKey16);
        addTestNode(17, _node17, _dkgPubkey17, _partialPublicKey17);
        addTestNode(18, _node18, _dkgPubkey18, _partialPublicKey18);
        addTestNode(19, _node19, _dkgPubkey19, _partialPublicKey19);
        addTestNode(20, _node20, _dkgPubkey20, _partialPublicKey20);
        addTestNode(21, _node21, _dkgPubkey21, _partialPublicKey21);
        addTestNode(22, _node22, _dkgPubkey22, _partialPublicKey22);
        addTestNode(23, _node23, _dkgPubkey23, _partialPublicKey23);
        addTestNode(24, _node24, _dkgPubkey24, _partialPublicKey24);
        addTestNode(25, _node25, _dkgPubkey25, _partialPublicKey25);
        addTestNode(26, _node26, _dkgPubkey26, _partialPublicKey26);
        addTestNode(27, _node27, _dkgPubkey27, _partialPublicKey27);
        addTestNode(28, _node28, _dkgPubkey28, _partialPublicKey28);
        addTestNode(29, _node29, _dkgPubkey29, _partialPublicKey29);
        addTestNode(30, _node30, _dkgPubkey30, _partialPublicKey30);
        addTestNode(31, _node31, _dkgPubkey31, _partialPublicKey31);

        // deal _owner and create _controller
        vm.deal(_owner, 1 * 10 ** 18);
        vm.deal(_stakingDeployer, 1 * 10 ** 18);

        vm.prank(_owner);
        _arpa = new ERC20("arpa token", "ARPA");

        // prepare operators for staking
        address[] memory operators = new address[](31);
        for (uint256 i = 1; i <= 31; i++) {
            operators[i - 1] = _testNodes[i].nodeAddress;
        }

        // deploy staking contract and add operators
        _prepareStakingContract(_stakingDeployer, address(_arpa), operators);

        // deploy _controller
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

    // * Test Node Setup
    function testNodeSetup() public {
        bytes memory dkgPublicKey;
        address nodeIdAddress;
        uint256[4] memory _publicKey;
        uint256[4] memory partialPublicKey;

        for (uint256 i = 1; i <= 31; i++) {
            nodeIdAddress = _testNodes[i].nodeAddress;
            dkgPublicKey = _testNodes[i]._publicKey;
            _publicKey = BLS.fromBytesPublicKey(dkgPublicKey);
            partialPublicKey = BLS.fromBytesPublicKey(_testNodes[i].partialPublicKey);
            assertEq(BLS.isValidPublicKey(_publicKey), true);
            assertEq(BLS.isValidPublicKey(partialPublicKey), true);
        }
    }

    // * Node Register Helper Testing
    mapping(uint256 => TestNode) internal _testNodes;

    struct TestNode {
        address nodeAddress;
        bytes _publicKey;
        bytes partialPublicKey;
    }

    // Add a test node to the _testNodes mapping and deal eth: Used for setup
    function addTestNode(uint256 index, address nodeAddress, bytes memory _publicKey, bytes memory partialPublicKey)
        public
    {
        TestNode memory newNode =
            TestNode({nodeAddress: nodeAddress, _publicKey: _publicKey, partialPublicKey: partialPublicKey});

        _testNodes[index] = newNode;
        vm.deal(nodeAddress, 1 * 10 ** 18);
    }

    // Take in a uint256 specifying node index, call node register using info from _testNodes mapping
    function registerIndex(uint256 nodeIndex) public {
        vm.prank(_testNodes[nodeIndex].nodeAddress);
        _controller.nodeRegister(_testNodes[nodeIndex]._publicKey);
    }

    // * Commit DKG Helper Functions
    struct Params {
        address nodeIdAddress;
        bool shouldRevert;
        bytes revertMessage;
        uint256 groupIndex;
        uint256 groupEpoch;
        bytes _publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
    }

    function dkgHelper(Params[] memory params) public {
        for (uint256 i = 0; i < params.length; i++) {
            vm.prank(params[i].nodeIdAddress);
            if (params[i].shouldRevert) {
                vm.expectRevert(params[i].revertMessage);
            }
            _controller.commitDkg(
                IController.CommitDkgParams(
                    params[i].groupIndex,
                    params[i].groupEpoch,
                    params[i]._publicKey,
                    params[i].partialPublicKey,
                    params[i].disqualifiedNodes
                )
            );
        }
    }

    // * ////////////////////////////////////////////////////////////////////////////////
    // * Extended Scenario Tests Begin (Rebalancing, Regrouping, Various Edgecases etc..)
    // * ////////////////////////////////////////////////////////////////////////////////

    //  Regroup remaining nodes after nodeQuit: (5 -> 4)
    function test5NodeQuit() public {
        /*
        group_0: 5 members
        1 member of group_0 wants to exit the network
        Then, _controller will let 4 members left in group_0 do dkg as 4 > 3 which is the threshold
        i.e. the group still meet the grouping condition
        after that, in happy path group_0 will be functional with 4 members.
        */

        // register nodes 1-5 using registerHelper()
        assertEq(_controller.getGroup(0).epoch, 0);
        registerIndex(1);
        registerIndex(2);
        registerIndex(3); // _controller emits event here
        assertEq(_controller.getGroup(0).epoch, 1); // g.epoch++
        registerIndex(4); // here
        assertEq(_controller.getGroup(0).epoch, 2); // g.epoch++
        registerIndex(5); // and here
        assertEq(_controller.getGroup(0).epoch, 3); // g.epoch++

        // group the 5 nodes using commitdkg.
        Params[] memory params = new Params[](5);
        bytes memory err;
        params[0] = Params(_node1, false, err, 0, 3, _publicKey, _partialPublicKey1, new address[](0));
        params[1] = Params(_node2, false, err, 0, 3, _publicKey, _partialPublicKey2, new address[](0));
        params[2] = Params(_node3, false, err, 0, 3, _publicKey, _partialPublicKey3, new address[](0));
        params[3] = Params(_node4, false, err, 0, 3, _publicKey, _partialPublicKey4, new address[](0));
        params[4] = Params(_node5, false, err, 0, 3, _publicKey, _partialPublicKey5, new address[](0));
        dkgHelper(params);

        // assert group info
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(_controller.getGroup(0).members.length, 5);
        assertEq(_controller.getGroup(0).size, 5);

        // node 1 calls nodeQuit
        vm.prank(_node1);
        _controller.nodeQuit(); // _controller emits event to start dkg proccess
        assertEq(_controller.getGroup(0).epoch, 4); // g.epoch++

        // node 2-4 call commitdkg
        params = new Params[](4);
        params[0] = Params(_node2, false, err, 0, 4, _publicKey, _partialPublicKey2, new address[](0));
        params[1] = Params(_node3, false, err, 0, 4, _publicKey, _partialPublicKey3, new address[](0));
        params[2] = Params(_node4, false, err, 0, 4, _publicKey, _partialPublicKey4, new address[](0));
        params[3] = Params(_node5, false, err, 0, 4, _publicKey, _partialPublicKey5, new address[](0));
        dkgHelper(params);

        // check group info
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(_controller.getGroup(0).members.length, 4);
        assertEq(_controller.getGroup(0).size, 4);
        // printGroupInfo(0);
    }

    //  Rebalance two groups after nodeQuit results in group falling below threshold (5,3) -> (3,4)
    function test53NodeQuit() public {
        /*
        group_0: 5 members
        group_1: 3 members
        1 member of group_1 wants to exist the network.
        Then, _controller will let group_1 which has 2 remaining members rebalance with group_0.
        Result: group_0 (3 members), group_1 (4 members) are both functional.
        */

        // * Register and group 5 nodes to group_0
        assertEq(_controller.getGroup(0).epoch, 0);
        registerIndex(1);
        registerIndex(2);
        registerIndex(3); // _controller emits event here (1-3 call commitDkg)
        assertEq(_controller.getGroup(0).epoch, 1); // g.epoch++
        registerIndex(4); // here (1-4 call commitDkg)
        assertEq(_controller.getGroup(0).epoch, 2); // g.epoch++
        registerIndex(5); // here
        assertEq(_controller.getGroup(0).epoch, 3); // g.epoch++

        // group the 5 nodes using commitdkg.
        Params[] memory params = new Params[](5);
        bytes memory err;
        params[0] = Params(_node1, false, err, 0, 3, _publicKey, _partialPublicKey1, new address[](0));
        params[1] = Params(_node2, false, err, 0, 3, _publicKey, _partialPublicKey2, new address[](0));
        params[2] = Params(_node3, false, err, 0, 3, _publicKey, _partialPublicKey3, new address[](0));
        params[3] = Params(_node4, false, err, 0, 3, _publicKey, _partialPublicKey4, new address[](0));
        params[4] = Params(_node5, false, err, 0, 3, _publicKey, _partialPublicKey5, new address[](0));
        dkgHelper(params);

        // assert group info
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(_controller.getGroup(0).members.length, 5);
        assertEq(_controller.getGroup(0).size, 5);

        // * Register and group 5 new nodes
        assertEq(_controller.getGroup(0).epoch, 3); // initial state
        assertEq(_controller.getGroup(1).epoch, 0); // initial state

        registerIndex(6); // Groups are rebalanced to (3,3) group_0 and group_1 epoch's are incremented here.
        // assertEq(_controller.getGroup(0).epoch, 4); // g.epoch++
        assertEq(_controller.getGroup(1).epoch, 1); // g.epoch++
        assertEq(_controller.getGroup(0).size, 3);
        assertEq(_controller.getGroup(1).size, 3);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(7); // added to group_0, only group_0 epoch is incremented
        assertEq(_controller.getGroup(0).epoch, 5); // g.epoch++
        assertEq(_controller.getGroup(1).epoch, 1); // no change
        assertEq(_controller.getGroup(0).size, 4); // g.size++
        assertEq(_controller.getGroup(1).size, 3);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(8); // added to group_1, only group_1 epoch is incremented
        assertEq(_controller.getGroup(0).epoch, 5); // no change
        assertEq(_controller.getGroup(1).epoch, 2); // g.epoch++
        assertEq(_controller.getGroup(0).size, 4);
        assertEq(_controller.getGroup(1).size, 4); // g.size++
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(9); // added to group_0, only group_0 epoch is incremented
        assertEq(_controller.getGroup(0).epoch, 6); // g.epoch++
        assertEq(_controller.getGroup(1).epoch, 2); // no change
        assertEq(_controller.getGroup(0).size, 5); // g.size++
        assertEq(_controller.getGroup(1).size, 4);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        registerIndex(10); // added to group_1, only group_1 epoch is incremented
        assertEq(_controller.getGroup(0).epoch, 6); // no change
        assertEq(_controller.getGroup(1).epoch, 3); // g.epoch++
        assertEq(_controller.getGroup(0).size, 5);
        assertEq(_controller.getGroup(1).size, 5); // g.size++
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);
        // groups have been reshuffled, current indexes are as follows:
        // group_0 (5,4,3,7,9), group_1 (6,1,2,8,10)

        // * Remove two nodes from group_1 (node8, node10) so that group_1 size == 3
        vm.prank(_node8);
        _controller.nodeQuit(); // group_1 epoch is incremented here
        assertEq(_controller.getGroup(1).epoch, 4); // g.epoch++
        assertEq(_controller.getGroup(1).size, 4); // g.size--

        vm.prank(_node10);
        _controller.nodeQuit(); // group_1 epoch is incremented here
        assertEq(_controller.getGroup(1).epoch, 5); // g.epoch++
        assertEq(_controller.getGroup(1).size, 3); // g.size--

        // * (5,3) configuration reached: group_0 (1,2,3,7,9) / group_1 (6,5,4)
        assertEq(_controller.getGroup(0).size, 5);
        assertEq(_controller.getGroup(1).size, 3);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);
        assertEq(_controller.getGroup(0).epoch, 6);
        assertEq(_controller.getGroup(1).epoch, 5);

        // * group group_0 and group_1 with commitDKG
        params = new Params[](5);
        params[0] = Params(_node1, false, err, 0, 6, _publicKey, _partialPublicKey5, new address[](0));
        params[1] = Params(_node2, false, err, 0, 6, _publicKey, _partialPublicKey4, new address[](0));
        params[2] = Params(_node3, false, err, 0, 6, _publicKey, _partialPublicKey3, new address[](0));
        params[3] = Params(_node7, false, err, 0, 6, _publicKey, _partialPublicKey7, new address[](0));
        params[4] = Params(_node9, false, err, 0, 6, _publicKey, _partialPublicKey9, new address[](0));
        dkgHelper(params);

        params = new Params[](3);
        params[0] = Params(_node6, false, err, 1, 5, _publicKey, _partialPublicKey6, new address[](0));
        params[1] = Params(_node5, false, err, 1, 5, _publicKey, _partialPublicKey1, new address[](0));
        params[2] = Params(_node4, false, err, 1, 5, _publicKey, _partialPublicKey2, new address[](0));
        dkgHelper(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), true);
        assertEq(_controller.getGroup(0).epoch, 6); // g.epoch++
        assertEq(_controller.getGroup(1).epoch, 5); // g.epoch++

        // * node in group_1 quits (node6)
        vm.prank(_node6);
        _controller.nodeQuit();
        // group_1 falls below threshold, rebalancing occurs to (3,4), event emitted for both groups
        assertEq(_controller.getGroup(0).epoch, 7); // g.epoch++
        assertEq(_controller.getGroup(1).epoch, 6); // g.epoch++
        assertEq(_controller.getGroup(0).size, 3);
        assertEq(_controller.getGroup(1).size, 4);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), false);

        // * group group_0 (1,2,3) and group_1 (4,5,9,7) with commitDKG
        params = new Params[](3);
        params[0] = Params(_node1, false, err, 0, 7, _publicKey, _partialPublicKey9, new address[](0));
        params[1] = Params(_node2, false, err, 0, 7, _publicKey, _partialPublicKey7, new address[](0));
        params[2] = Params(_node3, false, err, 0, 7, _publicKey, _partialPublicKey3, new address[](0));
        dkgHelper(params);

        params = new Params[](4);
        params[0] = Params(_node4, false, err, 1, 6, _publicKey, _partialPublicKey2, new address[](0));
        params[1] = Params(_node5, false, err, 1, 6, _publicKey, _partialPublicKey1, new address[](0));
        params[2] = Params(_node9, false, err, 1, 6, _publicKey, _partialPublicKey5, new address[](0));
        params[3] = Params(_node7, false, err, 1, 6, _publicKey, _partialPublicKey4, new address[](0));
        dkgHelper(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), true);

        // printGroupInfo(0);
        // printGroupInfo(1);
    }

    // * For the following tests we focus on Rebalancing logic rather than CommitDKG() details

    // [6,6] -> nodeRegister -> [3,6,4]
    function test66NodeRegister() public {
        /*
        group_0: 6 members
        group_1: 6 members
        A new node calls nodeRegister.
        _Controller should create a new group (group_2), add the new node into group_2, and then rebalance between group_0 and group_2.
        Final network status should be [3,6,4]
        */

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

        assertEq(_controller.getGroup(0).size, 6);
        assertEq(_controller.getGroup(1).size, 6);

        assertEq(_controller.getGroup(0).epoch, 8);
        assertEq(_controller.getGroup(1).epoch, 3);
        // Current state: group_0 (1,6,3,8,9,11), group_1 (7,2,5,4,10,12)

        // New node calls node register.
        registerIndex(13);

        assertEq(_controller.getGroup(0).size, 3);
        assertEq(_controller.getGroup(1).size, 6);
        assertEq(_controller.getGroup(2).size, 4);

        assertEq(_controller.getGroup(0).epoch, 9); // g.epoch++
        assertEq(_controller.getGroup(1).epoch, 3); // no change
        assertEq(_controller.getGroup(2).epoch, 1); // g.epoch++

        // Final State: group_0 (1,11,3), group_1 (7,2,5,4,10,12), group_2 (13,6,9,8)

        // printGroupInfo(0);
        // printGroupInfo(1);
        // printGroupInfo(2);
    }

    // [3,3] -> nodeRegister -> [3,3,1]
    function test33NodeRegister() public {
        /*
        group_0: 3 members
        group_1: 3 members
        A new node calls nodeRegister
        _Controller should create a new group_2, add the new node into group_2, then try to rebalance.
        Final network status should be (3,3,1) with group_2 not yet functional.
        */

        // Setup group_0 and group_1 so that they have 3 grouped nodes each.
        // register and group 3 nodes (1,2,3)
        registerIndex(1);
        registerIndex(2);
        registerIndex(3);
        assertEq(_controller.getGroup(0).epoch, 1);

        bytes memory err;
        Params[] memory params = new Params[](3);
        params[0] = Params(_node1, false, err, 0, 1, _publicKey, _partialPublicKey1, new address[](0));
        params[1] = Params(_node2, false, err, 0, 1, _publicKey, _partialPublicKey2, new address[](0));
        params[2] = Params(_node3, false, err, 0, 1, _publicKey, _partialPublicKey3, new address[](0));
        dkgHelper(params);

        // register and group 3 nodes (4,5,6)
        registerIndex(4);
        registerIndex(5);
        registerIndex(6);
        assertEq(_controller.getGroup(1).epoch, 1);

        params = new Params[](3);
        params[0] = Params(_node4, false, err, 1, 1, _publicKey, _partialPublicKey4, new address[](0));
        params[1] = Params(_node5, false, err, 1, 1, _publicKey, _partialPublicKey5, new address[](0));
        params[2] = Params(_node6, false, err, 1, 1, _publicKey, _partialPublicKey6, new address[](0));
        dkgHelper(params);

        // current state: group_0 (1,2,3), group_1 (4,5,6)

        // New node calls node register.
        registerIndex(7);

        // node added to new group 2. Group 0 and 1 remain functional, group 2 not yet funcional
        assertEq(_controller.getGroup(2).size, 1);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(2), false);

        // Final State: group_0 (1,2,3), group_1 (4,5,6), group_2 (7)
        // Group 2 is not yet functional, but it is created. Group 0 and 1 are functional.

        // printGroupInfo(0);
        // printGroupInfo(1);
        // printGroupInfo(2);
    }

    // Ideal number of groups of size 3 -> new nodeRegister()
    // [3,3,3,3,3] -> nodeRegister -> [4,3,3,3,3]
    function test33333NodeRegister() public {
        /*
        (5 groups) group 0-4 have 3 members each [3,3,3,3,3]
        A new node calls nodeRegister
        _Controller should add new node into group_0 resulting in network state [4,3,3,3,3].
        */

        // Setup groups 0,1,2,3,4 so that they have 3 grouped nodes each.
        // group_0: register and group 3 nodes (1,2,3)
        registerIndex(1);
        registerIndex(2);
        registerIndex(3);
        assertEq(_controller.getGroup(0).epoch, 1);
        bytes memory err;
        Params[] memory params = new Params[](3);
        params[0] = Params(_node1, false, err, 0, 1, _publicKey, _partialPublicKey1, new address[](0));
        params[1] = Params(_node2, false, err, 0, 1, _publicKey, _partialPublicKey2, new address[](0));
        params[2] = Params(_node3, false, err, 0, 1, _publicKey, _partialPublicKey3, new address[](0));
        dkgHelper(params);

        // group_1: register and group 3 nodes (4,5,6)
        registerIndex(4);
        registerIndex(5);
        registerIndex(6);
        assertEq(_controller.getGroup(1).epoch, 1);
        params = new Params[](3);
        params[0] = Params(_node4, false, err, 1, 1, _publicKey, _partialPublicKey4, new address[](0));
        params[1] = Params(_node5, false, err, 1, 1, _publicKey, _partialPublicKey5, new address[](0));
        params[2] = Params(_node6, false, err, 1, 1, _publicKey, _partialPublicKey6, new address[](0));
        dkgHelper(params);

        // group_2: register and group 3 nodes (7,8,9)
        registerIndex(7);
        registerIndex(8);
        registerIndex(9);
        assertEq(_controller.getGroup(2).epoch, 1);
        params = new Params[](3);
        params[0] = Params(_node7, false, err, 2, 1, _publicKey, _partialPublicKey7, new address[](0));
        params[1] = Params(_node8, false, err, 2, 1, _publicKey, _partialPublicKey8, new address[](0));
        params[2] = Params(_node9, false, err, 2, 1, _publicKey, _partialPublicKey9, new address[](0));
        dkgHelper(params);

        // group_3: register and group 3 nodes (10,11,12)
        registerIndex(10);
        registerIndex(11);
        registerIndex(12);
        assertEq(_controller.getGroup(3).epoch, 1);
        params = new Params[](3);
        params[0] = Params(_node10, false, err, 3, 1, _publicKey, _partialPublicKey10, new address[](0));
        params[1] = Params(_node11, false, err, 3, 1, _publicKey, _partialPublicKey11, new address[](0));
        params[2] = Params(_node12, false, err, 3, 1, _publicKey, _partialPublicKey12, new address[](0));
        dkgHelper(params);

        // group_4: register and group 3 nodes (13,14,15)
        registerIndex(13);
        registerIndex(14);
        registerIndex(15);
        assertEq(_controller.getGroup(4).epoch, 1);
        params = new Params[](3);
        params[0] = Params(_node13, false, err, 4, 1, _publicKey, _partialPublicKey13, new address[](0));
        params[1] = Params(_node14, false, err, 4, 1, _publicKey, _partialPublicKey14, new address[](0));
        params[2] = Params(_node15, false, err, 4, 1, _publicKey, _partialPublicKey15, new address[](0));
        dkgHelper(params);

        // current state: group_0 (1,2,3), group_1 (4,5,6), group_2 (7,8,9), group_3 (10,11,12), group_4 (13,14,15)
        assertEq(_controller.getGroup(0).size, 3);
        assertEq(_controller.getGroup(1).size, 3);
        assertEq(_controller.getGroup(2).size, 3);
        assertEq(_controller.getGroup(3).size, 3);
        assertEq(_controller.getGroup(4).size, 3);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(1), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(2), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(3), true);
        assertEq(checkIsStrictlyMajorityConsensusReached(4), true);

        // New node calls node register: It gets added to group_0, new event is emitted
        registerIndex(16);
        assertEq(_controller.getGroup(0).size, 4);
        assertEq(checkIsStrictlyMajorityConsensusReached(0), false);
        assertEq(_controller.getGroup(0).epoch, 2);

        // Final State: [4,3,3,3,3]
        // group_0 (1,2,3,16), group_1 (4,5,6), group_2 (7,8,9), group_3 (10,11,12), group_4 (13,14,15)
        // group_0 nonfunctiona, waiting to be grouped by commitDKG. Remaining groups are functional

        // printGroupInfo(0);
        // printGroupInfo(1);
        // printGroupInfo(2);
        // printGroupInfo(3);
        // printGroupInfo(4);
    }

    // ideal number of groups at max capacity -> nodeQuit()
    // [6,6,6,6,6] -> nodeRegister -> [3,6,6,6,6,4]]
    function test66666NodeRegister() public {
        /*
        (5 groups) group 0-4 have 6 members each [6,6,6,6,6]
        A new node calls node register.
        _Controller should create a new group (group_5), add the new node to group_5, and then rebalance between group_0 and group_5.
        Resulting network state should be [3,6,6,6,6,4]
        */

        // setup groups 0,1,2,3,4,5 so that they have 6 nodes each
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
        registerIndex(13);
        registerIndex(14);
        registerIndex(15);
        registerIndex(16);
        registerIndex(17);
        registerIndex(18);
        registerIndex(19);
        registerIndex(20);
        registerIndex(21);
        registerIndex(22);
        registerIndex(23);
        registerIndex(24);
        registerIndex(25);
        registerIndex(26);
        registerIndex(27);
        registerIndex(28);
        registerIndex(29);
        registerIndex(30);

        // Epochs after registering 30 nodes
        assertEq(_controller.getGroup(0).epoch, 20);
        assertEq(_controller.getGroup(1).epoch, 3);
        assertEq(_controller.getGroup(2).epoch, 3);
        assertEq(_controller.getGroup(3).epoch, 3);
        assertEq(_controller.getGroup(4).epoch, 3);

        // All groups of size 6
        assertEq(_controller.getGroup(0).size, 6);
        assertEq(_controller.getGroup(1).size, 6);
        assertEq(_controller.getGroup(2).size, 6);
        assertEq(_controller.getGroup(3).size, 6);
        assertEq(_controller.getGroup(4).size, 6);

        // A new node calls node register
        // _Controller creates new group_5, adds the new node to it,
        // and then rebalances between group_0 and group_5 (3,6,6,6,6,4)
        registerIndex(31);
        assertEq(_controller.getGroup(0).size, 3);
        assertEq(_controller.getGroup(1).size, 6);
        assertEq(_controller.getGroup(2).size, 6);
        assertEq(_controller.getGroup(3).size, 6);
        assertEq(_controller.getGroup(4).size, 6);
        assertEq(_controller.getGroup(5).size, 4);

        // grouping events emitted for group_0 and group_5
        assertEq(_controller.getGroup(0).epoch, 21); //g.epoch++
        assertEq(_controller.getGroup(5).epoch, 1); //g.epoch++

        // final state: [3,6,6,6,6,4]

        // printGroupInfo(0);
        // printGroupInfo(1);
        // printGroupInfo(2);
        // printGroupInfo(3);
        // printGroupInfo(4);
        // printGroupInfo(5);
    }

    // [6,3] -> group_1 nodeQuit -> [4,4]
    function test63Group1NodeQuit() public {
        /*
        (2 groups) group 0 has 6 members, group 1 has 3 members [6,3]
        group_1 calls nodeQuit
        _Controller should remove the node from group_1, and then rebalance between group_0 and group_1.'
        Resulting network state should be [4,4]
        */

        // Register 12 nodes
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

        // Current state: group_0 (1,6,3,8,9,11), group_1 (7,2,5,4,10,12)

        assertEq(_controller.getGroup(0).size, 6);
        assertEq(_controller.getGroup(1).size, 6);
        assertEq(_controller.getGroup(0).epoch, 8);
        assertEq(_controller.getGroup(1).epoch, 3);

        // Reduce group_1 size to 3
        vm.prank(_node12);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(1).epoch, 4); //g.epoch++
        vm.prank(_node10);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(1).epoch, 5); //g.epoch++
        vm.prank(_node2);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(1).epoch, 6); //g.epoch++
        // size [6,3] reached
        // Current state: group_0 (1,6,3,8,9,11), group_1 (7,4,5)

        // node7 from group_1 calls nodeQuit
        // _controller rebalances between group_0 and group_1 resulting in [4,4]
        vm.prank(_node7);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(0).size, 4);
        assertEq(_controller.getGroup(1).size, 4);
        assertEq(_controller.getGroup(0).epoch, 9); //g.epoch++
        assertEq(_controller.getGroup(1).epoch, 7); //g.epoch++

        // final state: [4,4]
        // group_0 (1,11,3,8), group_1 (5,4,6,9)

        // printGroupInfo(0);
        // printGroupInfo(1);
    }

    function test63Group0NodeQuit() public {
        /*
        group_0: 6 members
        group_1: 3 members
        member in group_0 calls nodeQuit.
        Then, the _controller should emitDkgEvent for group_0 with the 5 remaining remmbers.
        Resulting network state should be [5,3]
        */

        // Register 12 nodes
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

        // Current state: group_0 (1,6,3,8,9,11), group_1 (7,2,5,4,10,12)
        assertEq(_controller.getGroup(0).size, 6);
        assertEq(_controller.getGroup(1).size, 6);
        assertEq(_controller.getGroup(0).epoch, 8);
        assertEq(_controller.getGroup(1).epoch, 3);

        // Reduce group_1 size to 3
        vm.prank(_node12);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(1).epoch, 4); //g.epoch++
        vm.prank(_node10);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(1).epoch, 5); //g.epoch++
        vm.prank(_node2);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(1).epoch, 6); //g.epoch++
        // size [6,3] reached
        // current state: group_0 (1,6,3,8,9,11), group_1 (7,4,5)

        // node11 from group_0 calls nodeQuit
        vm.prank(_node11);
        _controller.nodeQuit();
        assertEq(_controller.getGroup(0).size, 5);
        assertEq(_controller.getGroup(1).size, 3);
        assertEq(_controller.getGroup(0).epoch, 9); // g.epoch++
        assertEq(_controller.getGroup(1).epoch, 6); // no change

        // final state: [5,3]
        // group_0 (1,6,3,8,9), group_1 (7,4,5)

        // printGroupInfo(0);
        // printGroupInfo(1);
    }
}
