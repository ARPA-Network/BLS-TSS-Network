// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MockServiceManager {
    address public avsDirectory;
    
    constructor(address _avsDirectory) {
        avsDirectory = _avsDirectory;
    }
}