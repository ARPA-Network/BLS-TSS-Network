// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {RandcastTestHelper} from "./RandcastTestHelper.sol";
import {ISignatureUtils, IAVSDirectory} from "../src/interfaces/IAVSDirectory.sol";
import {ServiceManager} from "../src/eigenlayer/ServiceManager.sol";

contract ServiceManagerTest is RandcastTestHelper {
    address[] tempAddresses;
    ServiceManager testInstance;
    event OperatorSlashed(address indexed operator, uint256 stakingPenalty);
    function setUp() public {
        _prepareRandcastContracts();
        tempAddresses = new address[](2);
        tempAddresses[0] = _node2;
        tempAddresses[1] = _node3;
        testInstance = ServiceManager(address(_serviceManager));
    }

    function testWhitelisting() public {
        // When not owner, error and revert for both addToWhiteList, setWhiteListEnabled and removeFromWhiteList
        vm.startPrank(_node1);   
        vm.expectRevert("Ownable: caller is not the owner");
        testInstance.addToWhitelist(tempAddresses);

        vm.expectRevert("Ownable: caller is not the owner");
        testInstance.setWhitelistEnabled(true);

        vm.expectRevert("Ownable: caller is not the owner");
        testInstance.removeFromWhitelist(tempAddresses);

        vm.stopPrank();

        // When is owner, and call addToWhiteList, should expect whitelist to be updated accordingly
        vm.startPrank(_admin);
        testInstance.addToWhitelist(tempAddresses);
        assertTrue(testInstance.whitelist(_node2));
        assertTrue(testInstance.whitelist(_node3));

        // When is owner, and call removeFromWhiteList, should expect whitelist to be updated accordingly
        tempAddresses = new address[](1);
        tempAddresses[0] = _node2;
        testInstance.removeFromWhitelist(tempAddresses);
        assertFalse(testInstance.whitelist(_node2));
        assertTrue(testInstance.whitelist(_node3));
       
        // When is owner, and call setWhiteListEnabled, should expect _whileListEnabled to be updated accordingly
        testInstance.setWhitelistEnabled(true);
        assertTrue(testInstance.whitelistEnabled());

        testInstance.setWhitelistEnabled(false);
        assertFalse(testInstance.whitelistEnabled());

        vm.stopPrank();
    }

    function testStrategyAndWeightsManagement() public {
        // When not owner, error and revert for setStrategyAndWeights
        tempAddresses = new address[](1);
        uint256[] memory tempWeights = new uint256[](1);
        tempWeights[0] = 100;
        tempAddresses[0] = _node2;
        vm.prank(_node1);
        vm.expectRevert("Ownable: caller is not the owner");
        testInstance.setStrategyAndWeights(tempAddresses, tempWeights);

        // When is owner, and call setStrategyAndWeights
        // If data mismatch between _strategy and _strategyWeights, erro   event OperatorSlashed(address indexed operator, uint256 stakingPenalty);r and revert
        vm.startPrank(_admin);

        tempWeights = new uint256[](2);
        tempWeights[0] = 100;
        tempWeights[1] = 200;
        vm.expectRevert(abi.encodeWithSelector(ServiceManager.StrategyAndWeightsLengthMismatch.selector));
        testInstance.setStrategyAndWeights(tempAddresses, tempWeights);

        // Else, expect _strategy and _strategyWeights to be updated accordingly
        address[] memory strategies = new address[](2);
        strategies[0] = _node1;
        strategies[1] = _node2;
        tempWeights = new uint256[](2);
        tempWeights[0] = 100;
        tempWeights[1] = 200;

        testInstance.setStrategyAndWeights(strategies, tempWeights);

        address[] memory restakeableStrategies = testInstance.getRestakeableStrategies();
        assertEq(restakeableStrategies.length, 2);
        assertEq(restakeableStrategies[0], _node1);
        assertEq(restakeableStrategies[1], _node2);

        // Skip the final validation for now (TO-DO: Update RandcastTestHelper with real delegation manager logic later) 
        
        // After above, should expect getOperatorShares to return value accordingly
        // uint256 operatorShare = testInstance.getOperatorShare(_node1);
        // assertEq(operatorShare, 50);
        // operatorShare = testInstance.getOperatorShare(_node2);
        // assertEq(operatorShare, 100);

        // After above, should expect getOperatorRestakedStrategies to return
        // Empty list if delegation manager doesn't have operator shares
        // address[] memory restakedStrategies = testInstance.getOperatorRestakedStrategies(_node3);
        // assertEq(restakedStrategies.length, 0);

        // // Real list if delegation manager has operator shares
        // restakedStrategies = testInstance.getOperatorRestakedStrategies(_node1);
        // assertEq(restakedStrategies.length, 2);
        // assertEq(restakedStrategies[0], _node1);
        // assertEq(restakedStrategies[1], _node2);
        
        vm.stopPrank();
    }

    //TO-DO, update RandcastTestHelper for AVS and re-test
    // function testOperatorRegistrationAndDeregistration() public {
    //     // Check value of whitelistEnabled and whitelisted, 
    //     // If enabled but not whitelisted, revert and error
    //     vm.prank(address(_nodeRegistry));
    //     tempAddresses = new address[](1);
    //     tempAddresses[0] = _node1;
    //     assertTrue(testInstance.whitelistEnabled());

    //     vm.prank(address(_nodeRegistry));
    //     vm.expectRevert(abi.encodeWithSelector(ServiceManager.OperatorNotInWhitelist.selector));
    //     IServiceManager(address(_serviceManager)).registerOperator(_node1, _emptyOperatorSignature);

    //     vm.prank(address(_nodeRegistry));
    //     testInstance.addToWhitelist(tempAddresses);
    //     assertTrue(testInstance.whitelist(_node1));

    //     // Check onlyNodeRegistry, if not, revert and error
    //     vm.prank(_node1);
    //     testInstance.registerOperator(_node1, _emptyOperatorSignature);

    //     vm.prank(_node2);
    //     vm.expectRevert(abi.encodeWithSelector(ServiceManager.SenderNotNodeRegistry.selector));
    //     testInstance.registerOperator(_node2, _emptyOperatorSignature);
    // }

    function testOperatorSlashing() public {
        // Verify that the slashDelegationStaking function can be called by the `NodeRegistry` contract
        // and that it correctly emits the OperatorSlashed event        
        vm.prank(address(_nodeRegistry));
        vm.expectEmit(true, true, true, true);
        emit OperatorSlashed(_node1, 100);
        testInstance.slashDelegationStaking(_node1, 100);
    }
}