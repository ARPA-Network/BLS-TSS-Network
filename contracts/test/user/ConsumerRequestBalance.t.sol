// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {GetRandomNumberExample} from "Randcast-User-Contract/user/examples/GetRandomNumberExample.sol";
import {GetShuffledArrayExample} from "Randcast-User-Contract/user/examples/GetShuffledArrayExample.sol";
import {RollDiceExample, GeneralRandcastConsumerBase} from "Randcast-User-Contract/user/examples/RollDiceExample.sol";
import {AdvancedGetShuffledArrayExample} from "Randcast-User-Contract/user/examples/AdvancedGetShuffledArrayExample.sol";
import {GeneralRandcastConsumerBase} from "Randcast-User-Contract/user/GeneralRandcastConsumerBase.sol";
import {IAdapter, Adapter, RandcastTestHelper} from "../RandcastTestHelper.sol";

//solhint-disable-next-line max-states-count
contract ConsumerRequestBalanceTest is RandcastTestHelper {
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

        prepareAnAvailableGroup();
    }

    function testAdapterAddress() public {
        emit log_address(address(_adapter));
        assertEq(_rollDiceExample.adapter(), address(_adapter));
    }

    function testCannotRequestWithoutSubscription() public {
        deal(_user, 1 * 1e18);
        vm.startPrank(_user);

        uint32 bunch = 10;
        vm.expectRevert(GeneralRandcastConsumerBase.NoSubscriptionBound.selector);
        _rollDiceExample.rollDice(bunch);
    }

    function testCannotRequestWithoutEnoughBalance() public {
        deal(_user, 1 * 1e18);

        uint256 feweTHBalance = 1 * 1e10;

        _prepareSubscription(_user, address(_rollDiceExample), feweTHBalance);

        uint32 bunch = 10;
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(_user);
        _rollDiceExample.rollDice(bunch);
    }

    function testRequestGeneralExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(_user, 10 * 1e18);
        // (1e18 arpa wei/arpa) (wei/gas * gas) / (wei/arpa) = arpa wei
        // paymentNoFee = (1e18 *
        //     weiPerUnitGas *
        //     (gasExceptCallback + callbackGasUsed) /
        //     uint256(weiPerUnitArpa);
        // callbackGasUsed = 501728 gas
        // WeiPerUnitArpa = 1e12 wei/arpa
        // weiPerUnitGas = 1e9 wei/gas
        // flat fee = 250000 1e12 arpa wei
        // Actual: 904212000000000000000
        // Expected: 891728000000000000000
        uint256 expectedPayment = 1e9 * (uint256(_gasExceptCallback) + 501728);

        uint256 plentyOfEthBalance = 1e16;
        // prepare subId 2 for _rollDiceExample
        IAdapter(address(_adapter)).createSubscription();
        uint64 subId = _prepareSubscription(_user, address(_rollDiceExample), plentyOfEthBalance);

        uint32 bunch = 10;
        vm.prank(_user);
        bytes32 requestId = _rollDiceExample.rollDice(bunch);

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 10);

        (uint256 afterBalance, uint256 inflightCost) = _getBalance(subId);

        // the upper limit of delta is 5%
        // maxPercentDelta is an 18 decimal fixed point number, where 1e18 == 100%
        assertApproxEqRel(plentyOfEthBalance - afterBalance, expectedPayment, 1e18 / 20);
        // inflight cost should be 0 after fulfillment
        assertEq(inflightCost, 0);

        for (uint256 i = 0; i < _rollDiceExample.lengthOfDiceResults(); i++) {
            assertTrue(_rollDiceExample.diceResults(i) > 0 && _rollDiceExample.diceResults(i) <= 6);
        }
        assertEq(_rollDiceExample.lengthOfDiceResults(), bunch);
    }

    function testCannotRequestWithTooMuchInflightCost() public {
        deal(_user, 1 * 1e18);

        // give the balance just enough for one request
        // give more than 3 times actual payment since we estimate 3 times max gas fee
        // (501728+30000) + 50000 * (5-3) + 550000 + 9000*5 = 1226728
        uint256 someEthBalance = 1230 * 3 * 1e12;
        _prepareSubscription(_user, address(_rollDiceExample), someEthBalance);
        uint32 bunch = 10;
        vm.prank(_user);
        _rollDiceExample.rollDice(bunch);
        // now we have an inflight request, then try to request again in the next block
        vm.roll(block.number + 1);
        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(_user);
        _rollDiceExample.rollDice(bunch);
    }

    function testRequestAdvancedExampleWithEnoughBalanceThenSuccessfullyFulfill() public {
        deal(_user, 1 * 1e18);
        // 350000 + 50000 * (5-3) + 550000 + 9000*5 = 1045000
        uint256 plentyOfEthBalance = 1050e12;
        // prepare subId 4 for _advancedGetShuffledArrayExample
        IAdapter(address(_adapter)).createSubscription();
        IAdapter(address(_adapter)).createSubscription();
        IAdapter(address(_adapter)).createSubscription();
        uint64 subId = _prepareSubscription(_user, address(_advancedGetShuffledArrayExample), plentyOfEthBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 6;
        // just cover actual gasused
        uint32 callbackGasLimit = 350000;
        uint256 callbackMaxGasPrice = 1 * 1e9;

        vm.prank(_user);
        bytes32 requestId = _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );

        deal(_node1, 1 * 1e18);
        _fulfillRequest(_node1, requestId, 12);

        assertEq(_advancedGetShuffledArrayExample.lengthOfShuffleResults(), 1);

        for (uint256 k = 0; k < _advancedGetShuffledArrayExample.lengthOfShuffleResults(); k++) {
            for (uint256 i = 0; i < upper; i++) {
                assertTrue(
                    _advancedGetShuffledArrayExample.shuffleResults(k, i) >= 0
                        && _advancedGetShuffledArrayExample.shuffleResults(k, i) < upper
                );
            }
        }
    }

    function testCannotRequestAdvancedExampleWithTooHighCallbackGasLimitAndCallbackMaxGasFee() public {
        deal(_user, 1 * 1e18);

        uint256 someEthBalance = 1e18;
        uint64 subId = _prepareSubscription(_user, address(_advancedGetShuffledArrayExample), someEthBalance);

        uint32 upper = 10;
        uint256 seed = 42;
        uint16 requestConfirmations = 6;
        uint32 callbackGasLimit = 2e6;
        uint256 callbackMaxGasPrice = 1e3 * 1e9;
        // payment = 2e6 * 1e3 * 1e9 = 2e18

        vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
        vm.prank(_user);
        _advancedGetShuffledArrayExample.getRandomNumberThenGenerateShuffledArray(
            upper, subId, seed, requestConfirmations, callbackGasLimit, callbackMaxGasPrice
        );
    }
}
