// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "forge-std/Test.sol";
import {Coordinator} from "src/Coordinator.sol";

import "openzeppelin-contracts/contracts/utils/Strings.sol";

// Future Controller Implementation
// import {Cotroller} from "src/Controller.sol";

contract CoordinatorTest is Test {
    Coordinator coordinator;

    // Constructor Args
    uint256 PHASE_DURATION = 30;
    uint256 THRESHOLD = 3;

    // Create 3 members for initialize()
    address public controller = address(0xCAFEBABE);
    address public node1 = address(0x1);
    address public node2 = address(0x2);
    address public node3 = address(0x3);

    bytes pubkey1 = "0x123"; //! use more realistic sample key.
    bytes pubkey2 = "0x456";
    bytes pubkey3 = "0x789";

    // Member Info for 3 participants
    address[] nodes = [node1, node2, node3];
    bytes[] keys = [pubkey1, pubkey2, pubkey3];

    // Each phase's data is just an opaque blob of data from the smart contract's
    // perspective, so we'll just use a dummy data object
    bytes data =
        "0x2222222222222222222222222222222222222222222222222222222222222222";

    function setUp() public {
        vm.deal(controller, 1 * 10**18);
        vm.prank(controller);
        coordinator = new Coordinator(THRESHOLD, PHASE_DURATION);
    }

    function testGetParticipantsAndKeys() public {
        vm.prank(controller);
        coordinator.initialize(nodes, keys);

        // Test getParticipants and getBlsKeys after initialize()
        address[] memory participants = coordinator.getParticipants();
        (uint256 threshold, bytes[] memory blsKeys) = coordinator.getBlsKeys();
        assertEq(participants, nodes);
        for (uint256 i = 0; i < blsKeys.length; i++) {
            assertEq(blsKeys[i], keys[i]);
        }
        assertEq(threshold, 3);
    }

    function testOnlyOwnerCanInitialize() public {
        // Non-Owner can't initialize
        vm.expectRevert("Ownable: caller is not the owner");
        coordinator.initialize(nodes, keys);

        // Owner can initizlize
        vm.prank(controller);
        coordinator.initialize(nodes, keys);
    }

    function testInitializeOnlyWhenNotStarted() public {
        // Initialize with owner
        vm.startPrank(controller);
        coordinator.initialize(nodes, keys);

        // Initialize callable onlyWhenNotStarted
        vm.expectRevert("DKG has already started");
        coordinator.initialize(nodes, keys);
        vm.stopPrank();
    }

    function testOnlyRegisteredCanPublish() public {
        vm.prank(controller);
        coordinator.initialize(nodes, keys);

        // non-registered node can't publish
        vm.expectRevert("you are not registered!");
        coordinator.publish(data);

        // registered node publishes successfully
        vm.startPrank(node1);
        coordinator.publish(data);
        bytes[] memory shares = coordinator.getShares();
        assertEq(shares[0], data);
        vm.stopPrank();
    }

    function testDoublePublish() public {
        vm.prank(controller);
        coordinator.initialize(nodes, keys);
        uint256 startBlock = coordinator.startBlock();

        // Phase 1: Shares
        vm.startPrank(node1);
        coordinator.publish(data); // succesful share
        vm.expectRevert("you have already published your shares");
        coordinator.publish(data);

        // Phase 2: Responses
        vm.roll(startBlock + 1 + PHASE_DURATION);
        coordinator.publish(data); // succesful response
        vm.expectRevert("you have already published your responses");
        coordinator.publish(data);

        // Phase 3: Justifications
        vm.roll(startBlock + 1 + 2 * PHASE_DURATION);
        coordinator.publish(data); // succesful justification
        vm.expectRevert("you have already published your justifications");
        coordinator.publish(data);

        // DKG End
        vm.roll(startBlock + 1 + 3 * PHASE_DURATION);
        vm.expectRevert("DKG Publish has ended");
        coordinator.publish(data); // succesful justification
    }

    function testPhases() public {
        // Phase 0: Pre-initialize
        assertEq(coordinator.inPhase(), 0);

        // Initialize
        vm.prank(controller);
        coordinator.initialize(nodes, keys);
        uint256 startBlock = coordinator.startBlock();

        // Phase 1
        assertEq(coordinator.inPhase(), 1);
        // emit log("StartBlock: ");
        // emit log_uint(startBlock);
        // emit log("Blocks since start: ");
        // emit log_uint(block.number - startBlock);

        // Phase 2
        vm.roll(startBlock + 1 + PHASE_DURATION);
        assertEq(coordinator.inPhase(), 2);
        // emit log("Blocks since start: ");
        // emit log_uint(block.number - startBlock);

        // Phase 3
        vm.roll(startBlock + 1 + 2 * PHASE_DURATION);
        assertEq(coordinator.inPhase(), 3);
        // emit log("Blocks since start: ");
        // emit log_uint(block.number - startBlock);

        // Phase 4 : commit DKG
        vm.roll(startBlock + 1 + 3 * PHASE_DURATION);
        assertEq(coordinator.inPhase(), 4);

        // DKG End
        vm.roll(startBlock + 1 + 4 * PHASE_DURATION);
        assertEq(coordinator.inPhase(), -1);
    }

    function testEnd2End() public {
        assertEq(coordinator.inPhase(), 0);

        // Initialize
        vm.prank(controller);
        coordinator.initialize(nodes, keys);
        assertEq(coordinator.inPhase(), 1);
        uint256 startBlock = coordinator.startBlock();

        // Phase 1: Publish
        vm.startPrank(node2);
        bytes memory my_shares = "0xDEADBEEF";
        coordinator.publish(my_shares); // only node2 publishes
        bytes[] memory shares = coordinator.getShares();
        assertEq(shares[0], "");
        assertEq(shares[1], my_shares);

        // Phase 2: Responses
        vm.roll(startBlock + 1 + PHASE_DURATION);
        assertEq(coordinator.inPhase(), 2);
        bytes memory my_responses = "0xBABECAFE";
        coordinator.publish(my_responses); // only node2 publishes
        bytes[] memory responses = coordinator.getResponses();
        assertEq(responses[0], "");
        assertEq(responses[1], my_responses);

        // Phase 3: Justifications
        vm.roll(startBlock + 1 + 2 * PHASE_DURATION);
        assertEq(coordinator.inPhase(), 3);
        bytes memory my_justifications = "0x0DDBA11";
        coordinator.publish(my_justifications); // only node2 publishes
        bytes[] memory justifications = coordinator.getJustifications();
        assertEq(justifications[0], "");
        assertEq(justifications[1], my_justifications);
    }
}
