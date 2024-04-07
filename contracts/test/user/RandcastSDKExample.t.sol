// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {DrawLotteryExample} from "Randcast-User-Contract/user/examples/DrawLotteryExample.sol";
import {PickRarityExample} from "Randcast-User-Contract/user/examples/PickRarityExample.sol";
import {PickPropertyExample} from "Randcast-User-Contract/user/examples/PickPropertyExample.sol";
import {PickWinnerExample} from "Randcast-User-Contract/user/examples/PickWinnerExample.sol";
import {Adapter, RandcastTestHelper, AdapterForTest} from "../RandcastTestHelper.sol";

//solhint-disable-next-line max-states-count
contract RandcastSDKExampleTest is RandcastTestHelper {
    DrawLotteryExample internal _drawLotteryExample;
    PickRarityExample internal _pickRarityExample;
    PickPropertyExample internal _pickPropertyExample;
    PickWinnerExample internal _pickWinnerExample;

    function setUp() public {
        skip(1000);
        _prepareRandcastContracts();

        vm.prank(_user);
        _drawLotteryExample = new DrawLotteryExample(address(_adapter));

        vm.prank(_user);
        _pickRarityExample = new PickRarityExample(address(_adapter));

        vm.prank(_user);
        _pickPropertyExample = new PickPropertyExample(address(_adapter));

        vm.prank(_user);
        _pickWinnerExample = new PickWinnerExample(address(_adapter));

        uint256 plentyOfEthBalance = 1e6 * 1e18;

        _prepareSubscription(_admin, address(_drawLotteryExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_pickRarityExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_pickPropertyExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_pickWinnerExample), plentyOfEthBalance);

        prepareAnAvailableGroup();
    }

    function testDrawLottery() public {
        deal(_user, 1 * 1e18);

        uint32 ticketNumber = 10;
        uint32 winnerNumber = 2;

        vm.prank(_user);
        bytes32 requestId = _drawLotteryExample.getTickets(ticketNumber, winnerNumber);
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 13);

        uint256[] memory ticketResults = _drawLotteryExample.getTicketResults(requestId);
        for (uint256 i = 0; i < winnerNumber; i++) {
            emit log_uint(_drawLotteryExample.winnerResults(i));
            bool winnerInTickets = false;
            for (uint256 j = 0; j < ticketResults.length; j++) {
                if (_drawLotteryExample.winnerResults(i) == ticketResults[j]) {
                    winnerInTickets = true;
                    break;
                }
            }
            assertTrue(winnerInTickets);
        }
        assertEq(_drawLotteryExample.lengthOfWinnerResults(), winnerNumber);
    }

    function testPickRarity() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        bytes32 requestId = _pickRarityExample.getRarity();
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 14);
        assertTrue(_pickRarityExample.indexResult() < 5);
    }

    function testPickProperty() public {
        deal(_user, 1 * 1e18);

        vm.prank(_user);
        bytes32 requestId = _pickPropertyExample.getProperty();
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 15);
        assertTrue(_pickPropertyExample.indexResult() < 3);
    }

    function testPickWinner() public {
        deal(_user, 1 * 1e18);
        vm.prank(_user);
        bytes32 requestId = _pickWinnerExample.getWinner();
        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 16);
        assertTrue(_pickWinnerExample.indexResult() < 3);
    }
}
