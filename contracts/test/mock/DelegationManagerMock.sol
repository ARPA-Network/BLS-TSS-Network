// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.18;

import {IDelegationManager} from "../../src/interfaces/IDelegationManager.sol";

contract DelegationManagerMock is IDelegationManager {
    function operatorShares(address operator, address strategy) external view override returns (uint256) {
        return 100;
    }
}