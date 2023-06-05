// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Adapter} from "../../src/Adapter.sol";

contract MockUpgradedAdapter is Adapter {
    function version() public pure returns (uint256) {
        return 2;
    }
}
