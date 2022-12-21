// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;
pragma experimental ABIEncoderV2;

import "forge-std/Test.sol";
import {Coordinator} from "src/Coordinator.sol";
import {Controller} from "src/Controller.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";
import "src/ICoordinator.sol";

// Suggested usage: forge test --match-contract Controller -vv

contract ControllerTest is Test {
    Controller controller;

    uint256 PHASE_DURATION = 10;

    address public owner = address(0xC0FF33);

    // Nodes: To be Registered
    address public node1 = address(0x1);
    address public node2 = address(0x2);
    address public node3 = address(0x3);
    address public node4 = address(0x4);
    address public node5 = address(0x5);
    address public node6 = address(0x6);
    address public node7 = address(0x7);
    address public node8 = address(0x8);
    address public node9 = address(0x9);
    address public node10 = address(0xA);
    address public node11 = address(0xB);

    // Node Public Keys
    bytes pubkey1 = hex"DECADE01";
    bytes pubkey2 = hex"DECADE02";
    bytes pubkey3 = hex"DECADE03";
    bytes pubkey4 = hex"DECADE04";
    bytes pubkey5 = hex"DECADE05";
    bytes pubkey6 = hex"DECADE06";
    bytes pubkey7 = hex"DECADE07";
    bytes pubkey8 = hex"DECADE08";
    bytes pubkey9 = hex"DECADE09";
    bytes pubkey10 = hex"DECADE10";
    bytes pubkey11 = hex"DECADE11";

    // uint256 registerCount; // track number of registered nodes for tests

    function setUp() public {
        // deal nodes
        vm.deal(node1, 1 * 10**18);
        vm.deal(node2, 1 * 10**18);
        vm.deal(node3, 1 * 10**18);
        vm.deal(node4, 1 * 10**18);
        vm.deal(node5, 1 * 10**18);

        // deal owner and create controller
        vm.deal(owner, 1 * 10**18);
        vm.prank(owner);
        controller = new Controller();

        // registerCount = 0; // reset registered node count to 0
    }

    function testNodeRegister() public {
        // printNodeInfo(node1);
        vm.prank(node1);
        controller.nodeRegister(pubkey1);
        // printNodeInfo(node1);

        Controller.Node memory n = controller.getNode(node1);
        assertEq(n.idAddress, node1);
        assertEq(n.dkgPublicKey, pubkey1);
        assertEq(n.state, true);
        assertEq(n.pendingUntilBlock, 0);
        assertEq(n.staking, 50000);

        vm.expectRevert("Node is already registered");
        vm.prank(node1);
        controller.nodeRegister(pubkey1);
    }

    function testMinimumThreshold() public {
        uint256 min;
        min = controller.tMinimumThreshold(3);
        emit log_named_uint("min 3", min);
        assertEq(min, 2);
        min = controller.tMinimumThreshold(7);
        emit log_named_uint("min 7", min);
        assertEq(min, 4);
        min = controller.tMinimumThreshold(100);
        emit log_named_uint("min 100", min);
        assertEq(min, 51);
    }

    function testEmitGroupEvent() public {
        // * fail emit group event if group does not exist
        vm.expectRevert("Group does not exist");
        controller.tNonexistantGroup(0);

        // * Register Three nodes and see if group struct is well formed
        uint256 groupIndex = 1;
        // printGroupInfo(groupIndex);
        // printNodeInfo(node1);

        // Register Node 1
        vm.prank(node1);
        controller.nodeRegister(pubkey1);
        // printGroupInfo(groupIndex);
        // printNodeInfo(node1);

        // Register Node 2
        vm.prank(node2);
        controller.nodeRegister(pubkey2);
        // printGroupInfo(groupIndex);

        // Register Node 3
        vm.prank(node3);
        controller.nodeRegister(pubkey3);
        // printGroupInfo(groupIndex);

        // check group struct is correct
        Controller.Group memory g = controller.getGroup(groupIndex);
        assertEq(g.index, 1);
        assertEq(g.epoch, 1);
        assertEq(g.size, 3);
        assertEq(g.threshold, 3);
        assertEq(g.members.length, 3);

        // Verify node2 info is recorded in group.members[1]
        Controller.Member memory m = g.members[1];
        // printMemberInfo(groupIndex, 1);
        assertEq(m.index, 1);
        assertEq(m.nodeIdAddress, node2);
        // assertEq(m.partialPublicKey, TODO);

        address coordinatorAddress = controller.getCoordinator(groupIndex);
        // emit log_named_address("\nCoordinator", coordinatorAddress);
    }

    function mockFindOrCreateTargetGroup()
        public
        returns (
            uint256, //groupIndex
            bool // needsRebalance
        )
    {
        emit log("============");
        if (controller.groupCount() == 0) { // if group is empty, addgroup.
            emit log("if groupCount == 0:");
            return (1,true);
        }

        // get the group index of the group with the minimum size, as well as the min size
        uint256 indexOfMinSize;
        uint256 minSize = controller.GROUP_MAX_CAPACITY();
        for (uint256 i = 0; i < controller.groupCount(); i++) {
            Controller.Group memory g = controller.getGroup(i+1); //
            if (g.size < minSize) {
                minSize = g.size;
                indexOfMinSize = i+1; // ! because groupCount starts at 1, ruoshan please check this. 
            }
        }



        // Get length of list of all group indexes where group.isStrictlyMajorityConsensusReached == true
        uint256 validGroupCount = controller.validGroupIndices().length;
        emit log("else:");
        emit log_named_uint("indexOfMinSize", indexOfMinSize);
        emit log_named_uint("minSize", minSize);
        emit log_named_uint("validGroupCount", validGroupCount);

        // // check if valid group count < ideal_number_of_groups || minSize == group_max_capacity
        // // If either condition is met and the number of valid groups == group count, call add group and return (index of new group, true)
        if ((validGroupCount < controller.IDEAL_NUMBER_OF_GROUPS() || minSize == controller.GROUP_MAX_CAPACITY()) && (validGroupCount == controller.groupCount())) {
            emit log("validGroupCount < IDEAL_NUMBER_OF_GROUPS || minSize == GROUP_MAX_CAPACITY");
            return (1,true);
            // uint256 groupIndex = controller.addGroup();
            // return (groupIndex, true);
        }

        // // if none of the above conditions are met:
        // return (indexOfMinSize, false);
        emit log ("neither condition met:");
        // emit log_named_uint("indexOfMinSize", indexOfMinSize);
        // emit log_named_uint("minSize", minSize);
        return (indexOfMinSize, false);

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
        vm.prank(node4);
        controller.nodeRegister(pubkey4);
        emit log_named_uint("groupCount", controller.groupCount());
        printGroupInfo(2);
    }

    function testGetMemberIndex() public {
        uint256 groupIndex = 1;

        int256 memberIndex = controller.getMemberIndex(groupIndex, node1);
        assertEq(memberIndex, -1);

        testEmitGroupEvent();

        memberIndex = controller.getMemberIndex(groupIndex, node1);
        assertEq(memberIndex, 0);
    }

    function testCoordinatorPhase() public {
        testEmitGroupEvent();
        uint256 groupIndex = 1;
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();
        assertEq(coordinator.inPhase(), 1);
        vm.roll(startBlock + 1 + PHASE_DURATION);
        assertEq(coordinator.inPhase(), 2);
        vm.roll(startBlock + 1 + 2 * PHASE_DURATION);
        assertEq(coordinator.inPhase(), 3);
        vm.roll(startBlock + 1 + 3 * PHASE_DURATION);
        assertEq(coordinator.inPhase(), 4);
        vm.roll(startBlock + 1 + 4 * PHASE_DURATION);
        assertEq(coordinator.inPhase(), -1);
    }

    // ! new
    struct CommitDkgParams {
        uint256 groupIndex;
        uint256 groupEpoch;
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
    }

    function testCommitDkg() public {
        testEmitGroupEvent();

        uint256 groupIndex = 1;
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();

        uint256 groupEpoch = 1;
        bytes memory partialPublicKey = hex"DECADE";
        bytes memory publicKey = hex"C0FFEE";
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
        vm.expectRevert(
            "Caller Group epoch does not match controller Group epoch"
        );
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
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        controller.commitDkg(params);

        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);

        // printGroupInfo(groupIndex); // ! Print

        // Succesful Commit: Node 1
        vm.prank(node1);
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex); // ! Print


        //  Fail if CommitCache already contains PartialKey for this node
        vm.prank(node1); 
        vm.expectRevert(
            "CommitCache already contains PartialKey for this node"  // ! PublcKey, not PartialKey? Should we add partialkey to commitresult?
        );
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey,
            disqualifiedNodes
        );
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);

        // Succesful Commit: Node 2
        vm.prank(node2);
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            hex"DECADE22", // partial public key 2
            disqualifiedNodes
        );
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), false);
        // printGroupInfo(groupIndex); // ! Print


        // Succesful Commit: Node 3
        vm.prank(node3);
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            hex"DECADE33", // partial public key 2
            disqualifiedNodes
        );
        controller.commitDkg(params);
        assertEq(checkIsStrictlyMajorityConsensusReached(groupIndex), true);
        // printGroupInfo(groupIndex); // ! Print
    }

    function testChooseRandomlyFromIndices() public {

        uint64 lastOutput = 0x2222222222222222; 

        uint256[] memory indices = new uint256[](5);
        indices[0] = 0;
        indices[1] = 1;
        indices[2] = 2;
        indices[3] = 3; 
        indices[4] = 4;
        
        uint256[] memory chosenIndices = controller.chooseRandomlyFromIndices(lastOutput, indices, 3);

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

        address[] memory majorityMembers = controller.getNonDisqualifiedMajorityMembers(
            nodes,
            disqualifedNodes
        );

        assertEq(majorityMembers.length, 2);
    }

    function testIsPartialKeyRegistered() public {
        testEmitGroupEvent();
        uint256 groupIndex = 1;
        // bytes memory sampleKey = hex"DECADE";
        // assertEq(
        //   .  controller.PartialKeyRegistered(groupIndex, node1, sampleKey),
        //     false
        // );
        assertEq(controller.partialKeyRegistered(groupIndex, node1), false);
    }

    function checkIsStrictlyMajorityConsensusReached(uint256 groupIndex)
        public
        view
        returns (bool)
    {
        Controller.Group memory g = controller.getGroup(groupIndex);
        return g.isStrictlyMajorityConsensusReached;
    }

    function testPostProccessDkg() public {
        testEmitGroupEvent();
        uint256 groupIndex = 1;
        uint256 groupEpoch = 1;
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        ICoordinator coordinator = ICoordinator(coordinatorAddress);
        uint256 startBlock = coordinator.startBlock();

        vm.expectRevert("Group does not exist");
        controller.postProcessDkg(0, 0); //(groupIndex, groupEpoch))

        vm.prank(node4);
        vm.expectRevert("Node is not a member of the group");
        controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(node1);
        vm.expectRevert(
            "Caller Group epoch does not match Controller Group epoch"
        );
        controller.postProcessDkg(groupIndex, 0); //(groupIndex, groupEpoch))

        vm.prank(node1);
        vm.expectRevert("DKG still in progress");
        controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))

        // Succesful post proccess dkg
        vm.roll(startBlock + 1 + 4 * PHASE_DURATION); // Put the coordinator in phase
        vm.prank(node1);
        controller.postProcessDkg(groupIndex, groupEpoch); //(groupIndex, groupEpoch))

        // Self destruct cannot be tested in foundry at the moment:
        // https://github.com/foundry-rs/foundry/issues/1543
        // https://github.com/foundry-rs/foundry/issues/2844
        // assertEq(coordinator.inPhase(), -1);
    }

    // ! Helper function for debugging below
    function printGroupInfo(uint256 groupIndex) public {
        // emit log(
        //     string.concat(
        //         "\n",
        //         Strings.toString(registerCount++),
        //         " Nodes Registered:"
        //     )
        // );

        Controller.Group memory g = controller.getGroup(groupIndex);

        uint256 groupCount = controller.groupCount();
        emit log("--------------------");
        emit log_named_uint("printing group info for: groupIndex", groupIndex);
        emit log("--------------------");
        emit log_named_uint("Total groupCount", groupCount);
        emit log_named_uint("g.index", g.index);
        emit log_named_uint("g.epoch", g.epoch);
        emit log_named_uint("g.size", g.size);
        emit log_named_uint("g.threshold", g.threshold);
        emit log_named_uint("g.members.length", g.members.length);
        emit log_named_uint(
            "g.isStrictlyMajorityConsensusReached",
            g.isStrictlyMajorityConsensusReached ? 1 : 0
        );
        for (uint256 i = 0; i < g.members.length; i++) {
            emit log_named_address(
                string.concat(
                    "g.members[",
                    Strings.toString(i),
                    "].nodeIdAddress"
                ),
                g.members[i].nodeIdAddress
            );
            emit log_named_bytes(
                string.concat(
                    "g.members[",
                    Strings.toString(i),
                    "].partialPublicKey"
                ),
                g.members[i].partialPublicKey
            );
        }
        // print committers
        emit log_named_uint("g.committers.length", g.committers.length);
        for (uint256 i = 0; i < g.committers.length; i++) {
            emit log_named_address(
                string.concat("g.committers[", Strings.toString(i), "]"),
                g.committers[i]
            );
        }
        // print commit cache info
        emit log_named_uint(
            "g.commitCacheList.length",
            g.commitCacheList.length
        );
        for (uint256 i = 0; i < g.commitCacheList.length; i++) {
            // print commit result
            emit log_named_bytes(
                string.concat(
                    "g.commitCacheList[",
                    Strings.toString(i),
                    "].commitResult.publicKey"
                ),
                g.commitCacheList[i].commitResult.publicKey
            );
            for (
                uint256 j = 0;
                j < g.commitCacheList[i].nodeIdAddress.length;
                j++
            ) {
                emit log_named_address(
                    string.concat(
                        "g.commitCacheList[",
                        Strings.toString(i),
                        "].nodeIdAddress[",
                        Strings.toString(j),
                        "].nodeIdAddress"
                    ),
                    g.commitCacheList[i].nodeIdAddress[j]
                );
            }
        }
    }

    function printNodeInfo(address nodeAddress) public {
        // emit log(
        //     string.concat(
        //         "\n",
        //         Strings.toString(registerCount++),
        //         " Nodes Registered:"
        //     )
        // );

        Controller.Node memory n = controller.getNode(nodeAddress);

        emit log_named_address("n.idAddress", n.idAddress);
        emit log_named_bytes("n.dkgPublicKey", n.dkgPublicKey);
        emit log_named_string("n.state", Bool.toText(n.state));
        emit log_named_uint("n.pendingUntilBlock", n.pendingUntilBlock);
        emit log_named_uint("n.staking", n.staking);
    }

    function printMemberInfo(uint256 groupIndex, uint256 memberIndex) public {
        emit log(
            string.concat(
                "\nGroupIndex: ",
                Strings.toString(groupIndex),
                " MemberIndex: ",
                Strings.toString(memberIndex),
                ":"
            )
        );

        Controller.Member memory m = controller.getMember(
            groupIndex,
            memberIndex
        );

        emit log_named_uint("m.index", m.index);
        emit log_named_address("m.nodeIdAddress", m.nodeIdAddress);
        emit log_named_bytes("m.partialPublicKey", m.partialPublicKey);
    }




    ////// ! Solidity Playground

    // function testDynamicArray() public {
    //     uint256[] memory numbers = new uint256[](3);
    //     numbers[0] = 1;
    //     numbers[1] = 2;
    //     numbers[2] = 3;

    //     uint256[] memory badNumbers = new uint256[](1);
    //     badNumbers[0] = 2;

    //     uint256[] memory goodNumbers = new uint256[](numbers.length);

    //     emit log_named_uint("numbers.length", numbers.length);
    //     emit log_named_uint("badNumbers.length", badNumbers.length);
    //     emit log_named_uint("goodNumbers.length (before)", goodNumbers.length);

    //     uint256 goodNumbersLength = 0;

    //     for (uint256 i = 0; i < numbers.length; i++) {
    //         bool disqualified = false;
    //         for (uint256 j = 0; j < badNumbers.length; j++) {
    //             if (numbers[i] == badNumbers[j]) {
    //                 disqualified = true;
    //                 break;
    //             }
    //             else {
    //                 goodNumbers[goodNumbersLength] = numbers[i];
    //                 goodNumbersLength++;
    //             }
    //         }
    //     }
    //     // delete trailing zeros
    //     uint256[] memory output = new uint256[](goodNumbersLength);
    //     for (uint256 i = 0; i < goodNumbersLength; i++) {
    //         output[i] = goodNumbers[i];
    //     }


    //     emit log_named_uint("output.length", output.length);
        
    // }

}
//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// * Helper library for logging bool
// EX : emit log_named_string("n.state", Bool.toText(n.state));
library Bool {
    function toUInt256(bool x) internal pure returns (uint256 r) {
        assembly {
            r := x
        }
    }

    function toBool(uint256 x) internal pure returns (string memory r) {
        // x == 0 ? r = "False" : "True";
        if (x == 1) {
            r = "True";
        } else if (x == 0) {
            r = "False";
        } else {}
    }

    function toText(bool x) internal pure returns (string memory r) {
        uint256 inUint = toUInt256(x);
        string memory inString = toBool(inUint);
        r = inString;
    }
}


