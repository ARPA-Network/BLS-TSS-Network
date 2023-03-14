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

    function testDkgBasicTests() public {
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
        Params[] memory params2 = new Params[](6);
        params2[0] = Params(node1, false, err, 0, 3, publicKey, partialPublicKey1, new address[](0));

        err = bytes("CommitCache already contains PartialKey for this node");
        params2[1] = Params(node1, true, err, 0, 3, publicKey, partialPublicKey1, new address[](0));

        params2[2] = Params(node2, false, err, 0, 3, publicKey, partialPublicKey2, new address[](0));
        params2[3] = Params(node3, false, err, 0, 3, publicKey, partialPublicKey3, new address[](0));
        params2[4] = Params(node4, false, err, 0, 3, publicKey, partialPublicKey4, new address[](0));
        params2[5] = Params(node5, false, err, 0, 3, publicKey, partialPublicKey5, new address[](0));
        dkgHelper(params2);
    }
}
