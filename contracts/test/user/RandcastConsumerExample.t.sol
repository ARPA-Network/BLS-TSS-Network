// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GetRandomNumberExample} from "Randcast-User-Contract/user/examples/GetRandomNumberExample.sol";
import {GetShuffledArrayExample} from "Randcast-User-Contract/user/examples/GetShuffledArrayExample.sol";
import {RollDiceExample} from "Randcast-User-Contract/user/examples/RollDiceExample.sol";
import {AdvancedGetShuffledArrayExample} from "Randcast-User-Contract/user/examples/AdvancedGetShuffledArrayExample.sol";
import {Adapter, RandcastTestHelper, AdapterForTest} from "../RandcastTestHelper.sol";

//solhint-disable-next-line max-states-count
contract RandcastConsumerExampleTest is RandcastTestHelper {
    GetRandomNumberExample internal _getRandomNumberExample;
    GetShuffledArrayExample internal _getShuffledArrayExample;
    RollDiceExample internal _rollDiceExample;
    AdvancedGetShuffledArrayExample internal _advancedGetShuffledArrayExample;

    function setUp() public {
        skip(1000);
        _prepareRandcastContracts();

        vm.prank(_user);
        _getRandomNumberExample = new GetRandomNumberExample(address(_adapter));

        vm.prank(_user);
        _rollDiceExample = new RollDiceExample(address(_adapter));

        vm.prank(_user);
        _getShuffledArrayExample = new GetShuffledArrayExample(address(_adapter));

        vm.prank(_user);
        _advancedGetShuffledArrayExample = new AdvancedGetShuffledArrayExample(address(_adapter));

        uint256 plentyOfEthBalance = 1e6 * 1e18;

        _prepareSubscription(_admin, address(_getRandomNumberExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_rollDiceExample), plentyOfEthBalance);
        _prepareSubscription(_admin, address(_getShuffledArrayExample), plentyOfEthBalance);
        prepareAnAvailableGroup();
    }

    function testAdapterAddress() public {
        emit log_address(address(_adapter));
        assertEq(_getRandomNumberExample.adapter(), address(_adapter));
        assertEq(_rollDiceExample.adapter(), address(_adapter));
        assertEq(_getShuffledArrayExample.adapter(), address(_adapter));
    }

    function testGetRandomNumber() public {
        deal(_user, 1 * 1e18);

        uint32 times = 10;
        for (uint256 i = 0; i < times; i++) {
            vm.prank(_user);
            bytes32 requestId = _getRandomNumberExample.getRandomNumber();

            Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
            bytes memory rawSeed = abi.encodePacked(rd.seed);
            emit log_named_bytes("rawSeed", rawSeed);

            deal(_node1, 1 * 1e18);
            _fulfillRequest(_node1, requestId, i);

            vm.roll(block.number + 1);
        }

        for (uint256 i = 0; i < _getRandomNumberExample.lengthOfRandomnessResults(); i++) {
            emit log_uint(_getRandomNumberExample.randomnessResults(i));
        }
        assertEq(_getRandomNumberExample.lengthOfRandomnessResults(), times);
    }

    function testRollDice() public {
        deal(_user, 1 * 1e18);

        uint32 bunch = 10;
        vm.prank(_user);
        bytes32 requestId = _rollDiceExample.rollDice(bunch);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 10);

        for (uint256 i = 0; i < _rollDiceExample.lengthOfDiceResults(); i++) {
            emit log_uint(_rollDiceExample.diceResults(i));
            assertTrue(_rollDiceExample.diceResults(i) > 0 && _rollDiceExample.diceResults(i) <= 6);
        }
        assertEq(_rollDiceExample.lengthOfDiceResults(), bunch);
    }

    function testGetShuffledArray() public {
        deal(_user, 1 * 1e18);

        uint32 upper = 10;
        vm.prank(_user);
        bytes32 requestId = _getShuffledArrayExample.getShuffledArray(upper);

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 11);

        for (uint256 i = 0; i < upper; i++) {
            emit log_uint(_getShuffledArrayExample.shuffleResults(i));
            assertTrue(
                _getShuffledArrayExample.shuffleResults(i) >= 0 && _getShuffledArrayExample.shuffleResults(i) < upper
            );
        }
        assertEq(_getShuffledArrayExample.lengthOfShuffleResults(), upper);
    }

    function testAdvancedGetShuffledArray() public {
        uint256 plentyOfEthBalance = 1e6 * 1e18;
        uint64 subId = _prepareSubscription(_admin, address(_advancedGetShuffledArrayExample), plentyOfEthBalance);

        deal(_user, 1 * 1e18);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 6;
        uint32 rdGasLimit = 350000;
        uint256 rdMaxGasPrice = 1 * 1e9;

        vm.prank(_user);
        bytes32 requestId = _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, rdGasLimit, rdMaxGasPrice
        );

        Adapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);
        bytes memory rawSeed = abi.encodePacked(rd.seed);
        emit log_named_bytes("rawSeed", rawSeed);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 12);

        assertEq(_advancedGetShuffledArrayExample.lengthOfShuffleResults(), 1);

        for (uint256 k = 0; k < _advancedGetShuffledArrayExample.lengthOfShuffleResults(); k++) {
            for (uint256 i = 0; i < upper; i++) {
                emit log_uint(_advancedGetShuffledArrayExample.shuffleResults(k, i));
                assertTrue(
                    _advancedGetShuffledArrayExample.shuffleResults(k, i) >= 0
                        && _advancedGetShuffledArrayExample.shuffleResults(k, i) < upper
                );
            }
        }
    }
}
