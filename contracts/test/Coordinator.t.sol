// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Test} from "forge-std/Test.sol";
import {Coordinator} from "../src/Coordinator.sol";

import {Strings} from "openzeppelin-contracts/contracts/utils/Strings.sol";

contract CoordinatorTest is Test {
    Coordinator internal _coordinator;

    // Constructor Args
    uint256 public constant PHASE_DURATION = 10;
    uint256 public constant THRESHOLD = 3;

    // Create 3 members for initialize()
    address internal _controller = address(0xCAFEBABE);
    address internal _node1 = address(0x1);
    address internal _node2 = address(0x2);
    address internal _node3 = address(0x3);

    bytes internal _pubkey1 = "0x123"; //! use more realistic sample key.
    bytes internal _pubkey2 = "0x456";
    bytes internal _pubkey3 = "0x789";

    // Member Info for 3 participants
    address[] internal _nodes = [_node1, _node2, _node3];
    bytes[] internal _keys = [_pubkey1, _pubkey2, _pubkey3];

    // Each phase's _data is just an opaque blob of _data from the smart contract's
    // perspective, so we'll just use a dummy _data object
    bytes internal _data = "0x2222222222222222222222222222222222222222222222222222222222222222";

    function setUp() public {
        vm.deal(_controller, 1 * 10 ** 18);
        vm.prank(_controller);
        _coordinator = new Coordinator(THRESHOLD, PHASE_DURATION);
    }

    function testGetParticipantsAndKeys() public {
        vm.prank(_controller);
        _coordinator.initialize(_nodes, _keys);

        // Test getParticipants and getDkgKeys after initialize()
        address[] memory participants = _coordinator.getParticipants();
        (uint256 threshold, bytes[] memory dkgKeys) = _coordinator.getDkgKeys();
        assertEq(participants, _nodes);
        for (uint256 i = 0; i < dkgKeys.length; i++) {
            assertEq(dkgKeys[i], _keys[i]);
        }
        assertEq(threshold, 3);
    }

    function testOnlyOwnerCanInitialize() public {
        // Non-Owner can't initialize
        vm.expectRevert("Ownable: caller is not the owner");
        _coordinator.initialize(_nodes, _keys);

        // Owner can initizlize
        vm.prank(_controller);
        _coordinator.initialize(_nodes, _keys);
    }

    function testInitializeOnlyWhenNotStarted() public {
        // Initialize with owner
        vm.startPrank(_controller);
        _coordinator.initialize(_nodes, _keys);

        // Initialize callable onlyWhenNotStarted
        vm.expectRevert("DKG has already started");
        _coordinator.initialize(_nodes, _keys);
        vm.stopPrank();
    }

    function testOnlyGroupMemberCanPublish() public {
        vm.prank(_controller);
        _coordinator.initialize(_nodes, _keys);

        // non-registered node can't publish
        vm.expectRevert("you are not a group member!");
        _coordinator.publish(_data);

        // registered node publishes successfully
        vm.startPrank(_node1);
        _coordinator.publish(_data);
        bytes[] memory shares = _coordinator.getShares();
        assertEq(shares[0], _data);
        vm.stopPrank();
    }

    function testDoublePublish() public {
        vm.prank(_controller);
        _coordinator.initialize(_nodes, _keys);
        uint256 startBlock = _coordinator.startBlock();

        // Phase 1: Shares
        vm.startPrank(_node1);
        _coordinator.publish(_data); // succesful share
        vm.expectRevert("share existed");
        _coordinator.publish(_data);

        // Phase 2: Responses
        vm.roll(startBlock + 1 + PHASE_DURATION);
        _coordinator.publish(_data); // succesful response
        vm.expectRevert("response existed");
        _coordinator.publish(_data);

        // Phase 3: Justifications
        vm.roll(startBlock + 1 + 2 * PHASE_DURATION);
        _coordinator.publish(_data); // succesful justification
        vm.expectRevert("justification existed");
        _coordinator.publish(_data);

        // DKG End
        vm.roll(startBlock + 1 + 3 * PHASE_DURATION);
        vm.expectRevert("DKG Publish has ended");
        _coordinator.publish(_data); // succesful justification
    }

    function testPhases() public {
        // Phase 0: Pre-initialize
        assertEq(_coordinator.inPhase(), 0);

        // Initialize
        vm.prank(_controller);
        _coordinator.initialize(_nodes, _keys);
        uint256 startBlock = _coordinator.startBlock();

        assertEq(_coordinator.inPhase(), 1);
        vm.roll(startBlock + 1 + PHASE_DURATION);
        assertEq(_coordinator.inPhase(), 2);
        vm.roll(startBlock + 1 + 2 * PHASE_DURATION);
        assertEq(_coordinator.inPhase(), 3);
        vm.roll(startBlock + 1 + 3 * PHASE_DURATION);
        assertEq(_coordinator.inPhase(), 4);
        vm.roll(startBlock + 1 + 4 * PHASE_DURATION);
        assertEq(_coordinator.inPhase(), -1);
    }

    function testEnd2End() public {
        assertEq(_coordinator.inPhase(), 0);

        // Initialize
        vm.prank(_controller);
        _coordinator.initialize(_nodes, _keys);
        assertEq(_coordinator.inPhase(), 1);
        uint256 startBlock = _coordinator.startBlock();

        // Phase 1: Publish
        vm.startPrank(_node2);
        bytes memory myShares = "0xDEADBEEF";
        _coordinator.publish(myShares); // only _node2 publishes
        bytes[] memory shares = _coordinator.getShares();
        assertEq(shares[0], "");
        assertEq(shares[1], myShares);

        // Phase 2: Responses
        vm.roll(startBlock + 1 + PHASE_DURATION);
        assertEq(_coordinator.inPhase(), 2);
        bytes memory myResponses = "0xBABECAFE";
        _coordinator.publish(myResponses); // only _node2 publishes
        bytes[] memory responses = _coordinator.getResponses();
        assertEq(responses[0], "");
        assertEq(responses[1], myResponses);

        // Phase 3: Justifications
        vm.roll(startBlock + 1 + 2 * PHASE_DURATION);
        assertEq(_coordinator.inPhase(), 3);
        bytes memory myJustifications = "0x0DDBA11";
        _coordinator.publish(myJustifications); // only _node2 publishes
        bytes[] memory justifications = _coordinator.getJustifications();
        assertEq(justifications[0], "");
        assertEq(justifications[1], myJustifications);
    }

    function testSelfDestructOnlyOwner() public {
        vm.prank(_controller);
        _coordinator.initialize(_nodes, _keys);
        vm.expectRevert("Ownable: caller is not the owner");
        _coordinator.selfDestruct();
        vm.prank(_controller);
        _coordinator.selfDestruct();
    }
}
