// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IAdapter} from "../src/interfaces/IAdapter.sol";

contract SimpleAdapter {
    function requestRandomness(IAdapter.RandomnessRequestParams calldata) public virtual returns (bytes32) {
        return 0;
    }

    function getLastSubscription(address) public pure returns (uint64) {
        return 0;
    }
}
