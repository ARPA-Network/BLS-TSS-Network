// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";
import {IController} from "./interfaces/IController.sol";
import {IOPCrossDomainMessenger} from "./interfaces/IOPCrossDomainMessenger.sol";
import {IControllerOracle} from "./interfaces/IControllerOracle.sol";
import {IChainMessenger} from "./interfaces/IChainMessenger.sol";

contract BaseChainMessenger is Ownable, IChainMessenger {
    address private _controllerRelayer;
    address private _controllerOracle;
    IOPCrossDomainMessenger private _crossDomainMessenger;

    error WrongControllerRelayer();

    constructor(address controllerRelayer, address controllerOracle, address crossDomainMessenger) {
        _controllerRelayer = controllerRelayer;
        _controllerOracle = controllerOracle;
        _crossDomainMessenger = IOPCrossDomainMessenger(crossDomainMessenger);
    }

    function relayMessage(address committer, IController.Group memory group) external {
        if (msg.sender != _controllerRelayer) {
            revert WrongControllerRelayer();
        }
        // call portal of L2 chain on L1 to trigger message relay, e.g. for OP this is to call
        _crossDomainMessenger.sendMessage(
            _controllerOracle,
            abi.encodeWithSelector(IControllerOracle.updateGroup.selector, committer, group),
            // 20% more gas than the actual gas used
            (344270 + 157160 * uint32(group.size)) * 6 / 5
        );
    }

    function setControllerRelayer(address controllerRelayer) external onlyOwner {
        _controllerRelayer = controllerRelayer;
    }

    function setControllerOracle(address controllerOracle) external onlyOwner {
        _controllerOracle = controllerOracle;
    }

    function setCrossDomainMessenger(address crossDomainMessenger) external onlyOwner {
        _crossDomainMessenger = IOPCrossDomainMessenger(crossDomainMessenger);
    }
}
