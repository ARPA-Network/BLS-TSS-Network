// // SPDX-License-Identifier: MIT
// pragma solidity >=0.8.10;

// import "../../src/user/examples/RollDiceExample.sol";
// import "../../src/user/examples/AdvancedGetShuffledArrayExample.sol";
// import "../RandcastTestHelper.sol";

// contract ConsumerRequestBalanceTest is RandcastTestHelper {
//     RollDiceExample rollDiceExample;
//     AdvancedGetShuffledArrayExample advancedGetShuffledArrayExample;

//     function setUp() public {
//         skip(1000);
//         changePrank(admin);
//         arpa = new ERC20("arpa token", "ARPA");
//         oracle = new MockArpaEthOracle();
//         controller = new Controller(address(arpa), address(oracle));

//         rollDiceExample = new RollDiceExample(address(controller));
//         advancedGetShuffledArrayExample = new AdvancedGetShuffledArrayExample(
//             address(controller)
//         );

//         uint16 minimumRequestConfirmations = 3;
//         uint32 maxGasLimit = 2000000;
//         uint32 stalenessSeconds = 30;
//         uint32 gasAfterPaymentCalculation = 30000;
//         uint32 gasExceptCallback = 66000;
//         int256 fallbackWeiPerUnitArpa = 1e12;
//         controller.setConfig(
//             minimumRequestConfirmations,
//             maxGasLimit,
//             stalenessSeconds,
//             gasAfterPaymentCalculation,
//             gasExceptCallback,
//             fallbackWeiPerUnitArpa,
//             Adapter.FeeConfig(
//                 250000,
//                 250000,
//                 250000,
//                 250000,
//                 250000,
//                 0,
//                 0,
//                 0,
//                 0
//             )
//         );
//     }

//     function testControllerAddress() public {
//         emit log_address(address(controller));
//         assertEq(rollDiceExample.controller(), address(controller));
//     }

//     function testCannotRequestWithoutSubscription() public {
//         deal(user, 1 * 1e18);
//         changePrank(user);

//         uint32 bunch = 10;
//         vm.expectRevert(Adapter.InvalidSubscription.selector);
//         rollDiceExample.rollDice(bunch);
//     }

//     function testCannotRequestWithoutEnoughBalance() public {
//         deal(user, 1 * 1e18);
//         changePrank(user);

//         uint96 fewArpaBalance = 1 * 1e18;
//         deal(address(arpa), address(user), fewArpaBalance);
//         arpa.approve(address(controller), fewArpaBalance);
//         prepareSubscription(address(rollDiceExample), fewArpaBalance);

//         uint32 bunch = 10;
//         vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
//         rollDiceExample.rollDice(bunch);
//     }

//     function testRequestGeneralExampleWithEnoughBalanceThenSuccessfullyFulfill()
//         public
//     {
//         deal(user, 1 * 1e18);
//         changePrank(user);

//         // (1e18 arpa wei/arpa) (wei/gas * gas) / (wei/arpa) = arpa wei
//         // paymentNoFee = (1e18 *
//         //     weiPerUnitGas *
//         //     (gasExceptCallback + callbackGasUsed) /
//         //     uint256(weiPerUnitArpa);
//         // callbackGasUsed = 496748 gas
//         // WeiPerUnitArpa = 1e12 wei/arpa
//         // weiPerUnitGas = 1e9 wei/gas
//         // gasExceptCallback  = 66000 gas
//         // flat fee = 250000 1e12 arpa wei
//         // Expected: 562578000000000000000
//         // Actual: 562748000000000000000
//         uint256 expectedPayment = (1e18 * 1e9 * (66000 + 496748)) / 1e12;

//         uint96 plentyOfArpaBalance = 1e6 * 1e18;
//         deal(address(arpa), address(user), plentyOfArpaBalance);
//         arpa.approve(address(controller), plentyOfArpaBalance);
//         uint64 subId = prepareSubscription(
//             address(rollDiceExample),
//             plentyOfArpaBalance
//         );

//         uint32 bunch = 10;
//         bytes32 requestId = rollDiceExample.rollDice(bunch);

//         deal(node, 1 * 1e18);
//         changePrank(node);
//         fulfillRequest(requestId);

//         changePrank(user);
//         (uint96 afterBalance, uint96 inflightCost) = getBalance(subId);
//         // the upper limit of delta is 5%
//         // maxPercentDelta is an 18 decimal fixed point number, where 1e18 == 100%
//         assertApproxEqRel(
//             expectedPayment,
//             plentyOfArpaBalance - afterBalance,
//             1e18 / 20
//         );
//         // inflight cost should be 0 after fulfillment
//         assertEq(inflightCost, 0);

//         for (uint256 i = 0; i < rollDiceExample.lengthOfDiceResults(); i++) {
//             assertTrue(
//                 rollDiceExample.diceResults(i) > 0 &&
//                     rollDiceExample.diceResults(i) <= 6
//             );
//         }
//         assertEq(rollDiceExample.lengthOfDiceResults(), bunch);
//     }

//     function testCannotRequestWithTooMuchInflightCost() public {
//         deal(user, 1 * 1e18);
//         changePrank(user);

//         // give the balance just enough for one request
//         // give more than 3 times actual payment(562748000000000000000) since we estimate 3 times max gas fee
//         uint96 someArpaBalance = 2e3 * 1e18;
//         deal(address(arpa), address(user), someArpaBalance);
//         arpa.approve(address(controller), someArpaBalance);
//         prepareSubscription(address(rollDiceExample), someArpaBalance);
//         uint32 bunch = 10;
//         rollDiceExample.rollDice(bunch);
//         // now we have an inflight request, then try to request again in the next block
//         vm.roll(block.number + 1);
//         vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
//         rollDiceExample.rollDice(bunch);
//     }

//     function testRequestAdvancedExampleWithEnoughBalanceThenSuccessfullyFulfill()
//         public
//     {
//         deal(user, 1 * 1e18);
//         changePrank(user);

//         uint96 plentyOfArpaBalance = 1e6 * 1e18;
//         deal(address(arpa), address(user), plentyOfArpaBalance);
//         arpa.approve(address(controller), plentyOfArpaBalance);
//         uint64 subId = prepareSubscription(
//             address(advancedGetShuffledArrayExample),
//             plentyOfArpaBalance
//         );

//         uint32 upper = 10;
//         uint256 seed = 42;
//         uint16 requestConfirmations = 0;
//         // just cover actual gasused
//         uint256 callbackGasLimit = 260000;
//         uint256 callbackMaxGasPrice = 1 * 1e9;

//         bytes32 requestId = advancedGetShuffledArrayExample
//             .getRandomNumberThenGenerateShuffledArray(
//                 upper,
//                 subId,
//                 seed,
//                 requestConfirmations,
//                 callbackGasLimit,
//                 callbackMaxGasPrice
//             );

//         deal(node, 1 * 1e18);
//         changePrank(node);
//         fulfillRequest(requestId);

//         changePrank(user);

//         assertEq(advancedGetShuffledArrayExample.lengthOfShuffleResults(), 1);

//         for (
//             uint256 k = 0;
//             k < advancedGetShuffledArrayExample.lengthOfShuffleResults();
//             k++
//         ) {
//             for (uint256 i = 0; i < upper; i++) {
//                 assertTrue(
//                     advancedGetShuffledArrayExample.shuffleResults(k, i) >= 0 &&
//                         advancedGetShuffledArrayExample.shuffleResults(k, i) <
//                         upper
//                 );
//             }
//         }
//     }

//     function testCannotRequestAdvancedExampleWithTooHighCallbackGasLimitAndCallbackMaxGasFee()
//         public
//     {
//         deal(user, 1 * 1e18);
//         changePrank(user);

//         uint96 plentyOfArpaBalance = 1e6 * 1e18;
//         deal(address(arpa), address(user), plentyOfArpaBalance);
//         arpa.approve(address(controller), plentyOfArpaBalance);
//         uint64 subId = prepareSubscription(
//             address(advancedGetShuffledArrayExample),
//             plentyOfArpaBalance
//         );

//         uint32 upper = 10;
//         uint256 seed = 42;
//         uint16 requestConfirmations = 0;
//         uint256 callbackGasLimit = 2e6;
//         uint256 callbackMaxGasPrice = 1e3 * 1e9;

//         vm.expectRevert(Adapter.InsufficientBalanceWhenRequest.selector);
//         advancedGetShuffledArrayExample
//             .getRandomNumberThenGenerateShuffledArray(
//                 upper,
//                 subId,
//                 seed,
//                 requestConfirmations,
//                 callbackGasLimit,
//                 callbackMaxGasPrice
//             );
//     }
// }
