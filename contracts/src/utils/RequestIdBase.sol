// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

contract RequestIdBase {
    function _makeRandcastInputSeed(uint256 userSeed, uint64 subId, address requester, uint256 nonce)
        internal
        view
        returns (uint256)
    {
        return uint256(keccak256(abi.encode(block.chainid, userSeed, subId, requester, nonce)));
    }

    function _makeRequestId(uint256 inputSeed) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(inputSeed));
    }
}
