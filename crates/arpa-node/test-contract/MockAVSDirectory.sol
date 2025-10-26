// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MockAVSDirectory {
    bytes32 public constant OPERATOR_AVS_REGISTRATION_TYPEHASH = 
        keccak256("OperatorAVSRegistration(address operator,address avs,bytes32 salt,uint256 expiry)");
    
    bytes32 private constant _TYPE_HASH = 
        keccak256("EIP712Domain(string name,uint256 chainId,address verifyingContract)");
    
    function calculateOperatorAVSRegistrationDigestHash(
        address operator,
        address avs,
        bytes32 salt,
        uint256 expiry
    ) public view returns (bytes32) {
        return _calculateSignableDigest(
            keccak256(abi.encode(OPERATOR_AVS_REGISTRATION_TYPEHASH, operator, avs, salt, expiry))
        );
    }
    
    function _calculateSignableDigest(bytes32 structHash) internal view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", _domainSeparator(), structHash));
    }
    
    function _domainSeparator() internal view returns (bytes32) {
        return keccak256(abi.encode(
            _TYPE_HASH,
            keccak256("MockAVSDirectory"),
            block.chainid,
            address(this)
        ));
    }
}