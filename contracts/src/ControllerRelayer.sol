// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IController} from "./interfaces/IController.sol";
import {IChainMessenger} from "./interfaces/IChainMessenger.sol";
import {UUPSUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import {OwnableUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";

contract ControllerRelayer is UUPSUpgradeable, OwnableUpgradeable {
    mapping(uint256 => mapping(uint256 => uint256)) private _chainRelayRecord;
    mapping(uint256 => address) private _chainMessengers;
    IController private _controller;

    event GroupRelayed(
        uint256 epoch, uint256 indexed groupIndex, uint256 indexed groupEpoch, address indexed committer
    );
    event ChainMessengerSet(uint256 indexed chainId, address indexed chainMessenger);
    event ChainRelayRecordReset(uint256 indexed chainId, uint256 indexed groupIndex, uint256 groupEpoch);

    error AbsentChainMessenger(uint256 chainId);
    error GroupObsolete(uint256 groupIndex, uint256 relayedGroupEpoch, uint256 currentGroupEpoch);
    error GroupNotFinalized(uint256 groupIndex, uint256 groupEpoch);
    error GroupEpochTooHigh(uint256 chainId, uint256 groupIndex, uint256 groupEpoch);

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(address controller) public initializer {
        _controller = IController(controller);

        __Ownable_init();
    }

    // solhint-disable-next-line no-empty-blocks
    function _authorizeUpgrade(address) internal override onlyOwner {}

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

        emit ChainMessengerSet(chainId, chainMessenger);
    }

    function setController(address controller) external onlyOwner {
        _controller = IController(controller);
    }

    function resetChainRelayRecord(uint256 chainId, uint256 groupIndex, uint256 groupEpoch) external onlyOwner {
        if (_chainRelayRecord[chainId][groupIndex] < groupEpoch) {
            revert GroupEpochTooHigh(chainId, groupIndex, groupEpoch);
        }

        _chainRelayRecord[chainId][groupIndex] = groupEpoch;

        emit ChainRelayRecordReset(chainId, groupIndex, groupEpoch);
    }
}
