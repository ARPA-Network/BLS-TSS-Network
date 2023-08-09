// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IController} from "./interfaces/IController.sol";
import {IChainMessenger} from "./interfaces/IChainMessenger.sol";
import {Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";

contract ControllerRelayer is Ownable {
    mapping(uint256 => mapping(uint256 => uint256)) private _chainRelayRecord;

    mapping(uint256 => address) private _chainMessengers;
    IController private _controller;

    constructor(address controller) {
        _controller = IController(controller);
    }

    event GroupRelayed(
        uint256 epoch, uint256 indexed groupIndex, uint256 indexed groupEpoch, address indexed committer
    );

    error AbsentChainMessenger(uint256 chainId);
    error GroupObsolete(uint256 groupIndex, uint256 relayedGroupEpoch, uint256 currentGroupEpoch);
    error GroupNotFinalized(uint256 groupIndex, uint256 groupEpoch);

    function relayGroup(uint256 chainId, uint256 groupIndex) external {
        if (_chainMessengers[chainId] == address(0)) {
            revert AbsentChainMessenger(chainId);
        }

        IController.Group memory groupToRelay = _controller.getGroup(groupIndex);

        // need the group is not in a DKG process so that group info on current epoch is finalized
        if (_controller.getCoordinator(groupIndex) != address(0)) {
            revert GroupNotFinalized(groupIndex, groupToRelay.epoch);
        }

        if (_chainRelayRecord[chainId][groupIndex] >= groupToRelay.epoch) {
            revert GroupObsolete(groupIndex, groupToRelay.epoch, _chainRelayRecord[chainId][groupIndex]);
        }

        _chainRelayRecord[chainId][groupIndex] = groupToRelay.epoch;
        // call the messenger of corresponding chain
        IChainMessenger(_chainMessengers[chainId]).relayMessage(msg.sender, groupToRelay);

        emit GroupRelayed(_controller.getGroupEpoch(), groupIndex, groupToRelay.epoch, msg.sender);
    }

    function setChainMessenger(uint256 chainId, address chainMessenger) external onlyOwner {
        _chainMessengers[chainId] = chainMessenger;
    }

    function setController(address controller) external onlyOwner {
        _controller = IController(controller);
    }
}
