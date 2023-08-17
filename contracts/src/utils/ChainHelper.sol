// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

contract ChainHelper {
    function getBlockTime() public returns (uint256) {
        uint256 chainId;
        assembly { 
            chainId := chainid()
        }
        if(chainId == 1) { // mainnet
            return 12;
        } else if (chainId == 10) { // Optimism
            return 2;
        } else {
            revert("Unrecognized chainId");
        }
    }
}