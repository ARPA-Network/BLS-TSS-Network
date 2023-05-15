// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.15;

import "../../src/Adapter.sol";

contract MockUpgradedAdapter is Adapter {
    function version() public pure returns (uint256) {
        return 2;
    }
}
