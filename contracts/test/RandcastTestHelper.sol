// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "forge-std/Test.sol";
import "../src/interfaces/IAdapter.sol";
import "../src/Controller.sol";
import "./MockArpaEthOracle.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

abstract contract RandcastTestHelper is Test {
    Controller controller;
    MockArpaEthOracle oracle;
    IERC20 arpa;

    address public admin = address(0xABCD);
    address public user = address(0x11);
    address public node = address(0x22);

    // Nodes: To be Registered
    address public node1 = address(0x1);
    address public node2 = address(0x2);
    address public node3 = address(0x3);

    // Node DKG Communication Public Keys
    bytes pubkey1 = hex"DECADE01";
    bytes pubkey2 = hex"DECADE02";
    bytes pubkey3 = hex"DECADE03";

    // Node Partial Public Keys
    bytes partialPublicKey1 = hex"DECADE11";
    bytes partialPublicKey2 = hex"DECADE12";
    bytes partialPublicKey3 = hex"DECADE13";

    uint256 signature1 =
        0x2dcb14c407beb29593b6ee1d0db90642f95d23441fe7bb68f195c116563b5882;

    function fulfillRequest(bytes32 requestId) internal {
        Controller.Callback memory callback = controller.getPendingRequest(
            requestId
        );
        // mock confirmation times and SIGNATURE_TASK_EXCLUSIVE_WINDOW = 10;
        vm.roll(block.number + callback.requestConfirmations + 10);

        // mock fulfillRandomness directly
        Controller.PartialSignature[] memory mockPartialSignatures;
        controller.fulfillRandomness(
            0, // fixed group 0
            requestId,
            signature1,
            mockPartialSignatures
        );
    }

    function prepareSubscription(address consumer, uint96 balance)
        internal
        returns (uint64)
    {
        uint64 subId = controller.createSubscription();
        controller.fundSubscription(subId, balance);
        controller.addConsumer(subId, consumer);
        return subId;
    }

    function getBalance(uint64 subId) internal view returns (uint96, uint96) {
        (uint96 balance, uint96 inflightCost, , , ) = controller
            .getSubscription(subId);
        return (balance, inflightCost);
    }

    function prepareAnAvailableGroup() public {
        vm.stopPrank();

        // deal nodes
        vm.deal(node1, 1 * 10**18);
        vm.deal(node2, 1 * 10**18);
        vm.deal(node3, 1 * 10**18);

        // Register Node 1
        vm.broadcast(node1);
        controller.nodeRegister(pubkey1);

        // Register Node 2
        vm.broadcast(node2);
        controller.nodeRegister(pubkey2);

        // Register Node 3
        vm.broadcast(node3);
        controller.nodeRegister(pubkey3);

        uint256 groupIndex = 0;
        uint256 groupEpoch = 1;

        bytes memory publicKey = abi.encodePacked(
            uint256(
                0x1a507c593ab755ddc738a62bb1edbf00de9d2e0f6829a663c53fa281ca3a296b
            ),
            uint256(
                0x17bfa426fe907fb295063261d2348ad72f3b40c1aaeb8a0e31e29b341d9cc14f
            ),
            uint256(
                0x247fe0adc753328cb9250964f16b77693d273892270be5cfbb4aca3b625606cc
            ),
            uint256(
                0x17e4867e1df6f439500568aaa567952b5c47f3b4eb3a824fcee17000917ce1d0
            )
        );
        address[] memory disqualifiedNodes = new address[](0);
        Controller.CommitDkgParams memory params;

        // Succesful Commit: Node 1
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey1,
            disqualifiedNodes
        );
        vm.broadcast(node1);
        controller.commitDkg(params);

        // Succesful Commit: Node 2
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey2,
            disqualifiedNodes
        );
        vm.broadcast(node2);
        controller.commitDkg(params);

        // Succesful Commit: Node 3
        params = Controller.CommitDkgParams(
            groupIndex,
            groupEpoch,
            publicKey,
            partialPublicKey3,
            disqualifiedNodes
        );
        vm.broadcast(node3);
        controller.commitDkg(params);
    }
}
