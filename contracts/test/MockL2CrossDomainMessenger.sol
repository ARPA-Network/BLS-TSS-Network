// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

contract MockL2CrossDomainMessenger {
    address private _xDomainMessageSender;

    constructor(address __xDomainMessageSender) {
        _xDomainMessageSender = __xDomainMessageSender;
    }

    function xDomainMessageSender() external view returns (address) {
        return _xDomainMessageSender;
    }
}
