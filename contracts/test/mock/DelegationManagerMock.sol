// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.18;

import {IDelegationManager} from "../../src/interfaces/IDelegationManager.sol";

contract DelegationManagerMock is IDelegationManager {
    uint256 share = 500_00 * 1e18;

    function setShares(uint256 _share) external {
        share = _share;
    }

    function operatorShares(address operator, address strategy) external view override returns (uint256) {
        return share;
    }
}