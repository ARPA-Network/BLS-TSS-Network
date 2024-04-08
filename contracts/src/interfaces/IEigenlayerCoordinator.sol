// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.18;

import {ISignatureUtils} from "./ISignatureUtils.sol";

interface IEigenlayerCoordinator {
    function registerOperator(address operator, ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature)
        external;

    function deregisterOperator(address operator) external;

    function slashDelegationStaking(address operator, uint256 amount) external;

    function getOperatorShare(address operator) external view returns (uint256);
}
