// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

contract ChainHelper {
    function getBlockTime() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (chainId == 1) {
            // mainnet
            return 12;
        } else if (chainId == 10) {
            // Optimism
            return 2;
        } else {
            revert("Unrecognized chainId");
        }
    }
}
