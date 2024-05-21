// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.18;

import {ISignatureUtils} from "../../src/interfaces/ISignatureUtils.sol";
import {IAVSDirectory} from "../../src/interfaces/IAVSDirectory.sol";

contract AVSDirectoryMock is IAVSDirectory {
    event RegisterOperatorToAVSCalled(address operator, ISignatureUtils.SignatureWithSaltAndExpiry operatorSignature);
    event DeregisterOperatorFromAVSCalled(address operator);
    event UpdateAVSMetadataURICalled(string metadataURI);
    event OperatorSaltIsSpentCalled(address operator, bytes32 salt);
    event CalculateOperatorAVSRegistrationDigestHashCalled(address operator, address avs, bytes32 salt, uint256 expiry);
    event OperatorAVSRegistrationTypehashCalled();

    function registerOperatorToAVS(
        address operator,
        ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature
    ) external {
        emit RegisterOperatorToAVSCalled(operator, operatorSignature);
    }

    function deregisterOperatorFromAVS(address operator) external {
        emit DeregisterOperatorFromAVSCalled(operator);
    }

    function updateAVSMetadataURI(string calldata metadataURI) external {
        emit UpdateAVSMetadataURICalled(metadataURI);
    }

    function operatorSaltIsSpent(address operator, bytes32 salt) external view returns (bool) {
        return false;
    }

    function calculateOperatorAVSRegistrationDigestHash(address operator, address avs, bytes32 salt, uint256 expiry)
        external
        view
        returns (bytes32)
    {
        return bytes32(0);
    }

    function OPERATOR_AVS_REGISTRATION_TYPEHASH() external view returns (bytes32) {
        return bytes32(0);
    }
}