// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Adapter} from "../src/Adapter.sol";

contract AdapterForTest is Adapter {
    mapping(bytes32 => RequestDetail) internal _requestDetails;

    function requestRandomness(RandomnessRequestParams calldata p) public override returns (bytes32) {
        bytes32 requestId = super.requestRandomness(p);
        uint256 rawSeed =
            _makeRandcastInputSeed(p.seed, p.subId, msg.sender, _consumers[msg.sender].nonces[p.subId] - 1);

        // Record RequestDetail struct
        RequestDetail storage rd = _requestDetails[requestId];
        rd.subId = p.subId;
        rd.groupIndex = _lastAssignedGroupIndex;
        rd.requestType = p.requestType;
        rd.params = p.params;
        rd.callbackContract = msg.sender;
        rd.seed = rawSeed;
        rd.requestConfirmations = p.requestConfirmations;
        rd.callbackGasLimit = p.callbackGasLimit;
        rd.callbackMaxGasPrice = p.callbackMaxGasPrice;
        rd.blockNum = block.number;

        return requestId;
    }

    function getPendingRequest(bytes32 requestId) public view returns (RequestDetail memory) {
        return _requestDetails[requestId];
    }

    function getInflightCost(uint64 subId, bytes32 requestId) public view returns (uint256) {
        return _subscriptions[subId].inflightPayments[requestId];
    }
}
