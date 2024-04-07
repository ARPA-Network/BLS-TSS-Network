// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {SharedConsumer} from "Randcast-User-Contract/user/SharedConsumer.sol";
import {ISharedConsumer} from "Randcast-User-Contract/interfaces/ISharedConsumer.sol";
import {Adapter, RandcastTestHelper, AdapterForTest, ERC1967Proxy} from "../RandcastTestHelper.sol";

//solhint-disable-next-line max-states-count
contract RandcastPlaygroundConsumerTest is RandcastTestHelper {
    ERC1967Proxy internal _shareConsumer;

    function setUp() public {
        skip(1000);
        _prepareRandcastContracts();

        vm.prank(_user);
        SharedConsumer _shareConsumerImpl = new SharedConsumer(address(_adapter));

        vm.prank(_user);
        _shareConsumer = new ERC1967Proxy(address(_shareConsumerImpl), abi.encodeWithSignature("initialize()"));

        uint256 plentyOfEthBalance = 1e6 * 1e18;
        _prepareSubscription(_admin, address(_shareConsumer), plentyOfEthBalance);

        prepareAnAvailableGroup();
    }

    function testPlaygroundDrawTickets() public {
        deal(_user, 1 * 1e18);
        vm.prank(_user);
        ISharedConsumer(address(_shareConsumer)).setTrialSubscription(5);

        uint32 ticketNumber = 30;
        uint32 winnerNumber = 10;

        vm.prank(_user);
        uint256 gasFee = ISharedConsumer(address(_shareConsumer)).estimateFee(
            ISharedConsumer.PlayType.Draw, 0, abi.encode(ticketNumber, winnerNumber)
        );

        vm.prank(_user);
        bytes32 requestId =
            ISharedConsumer(address(_shareConsumer)).drawTickets{value: gasFee}(ticketNumber, winnerNumber, 0, 0, 6);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 17);

        (,, uint256 balance,,,,,,) = AdapterForTest(address(_adapter)).getSubscription(2);
        vm.prank(_user);
        uint256 curBalance = _user.balance;
        ISharedConsumer(address(_shareConsumer)).cancelSubscription();
        assertEq(_user.balance, curBalance + balance);
        emit log_uint(gasFee);
    }

    function testPlaygroundRollDice() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        ISharedConsumer(address(_shareConsumer)).setTrialSubscription(5);

        uint32 bunch = 1;
        uint32 size = 6;
        vm.prank(_user);
        uint256 gasFee = ISharedConsumer(address(_shareConsumer)).estimateFee(
            ISharedConsumer.PlayType.Roll, 0, abi.encode(bunch, size)
        );

        vm.prank(_user);
        bytes32 requestId = ISharedConsumer(address(_shareConsumer)).rollDice{value: gasFee}(bunch, size, 0, 0, 0);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 17);

        (,, uint256 balance,,,,,,) = AdapterForTest(address(_adapter)).getSubscription(2);
        vm.prank(_user);
        uint256 curBalance = _user.balance;
        ISharedConsumer(address(_shareConsumer)).cancelSubscription();
        assertEq(_user.balance, curBalance + balance);
        emit log_uint(gasFee);
    }

    function testUseSharedSubscription() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        ISharedConsumer(address(_shareConsumer)).setTrialSubscription(1);

        uint32 ticketNumber = 30;
        uint32 winnerNumber = 1;
        vm.prank(_user);
        bytes32 requestId = ISharedConsumer(address(_shareConsumer)).drawTickets(ticketNumber, winnerNumber, 1, 0, 0);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 18);
    }
}
