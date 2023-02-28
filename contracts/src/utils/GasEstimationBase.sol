// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "./StringAndUintConverter.sol";

contract GasEstimationBase {
    /**
     * @notice Estimates gas used by actually calling that function then reverting with the gas used as string
     * @param to Destination address
     * @param value Ether value
     * @param data Data payload
     */
    function requiredTxGas(address to, uint256 value, bytes calldata data) external returns (uint256) {
        uint256 startGas = gasleft();
        // We don't provide an error message here, as we use it to return the estimate
        // solium-disable-next-line error-reason
        require(executeCall(to, value, data, gasleft()));
        uint256 requiredGas = startGas - gasleft();
        string memory s = uintToString(requiredGas);
        // Convert response to string and return via error message
        revert(s);
    }

    function executeCall(address to, uint256 value, bytes memory data, uint256 txGas) internal returns (bool success) {
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            success := call(txGas, to, value, add(data, 0x20), mload(data), 0, 0)
        }
    }

    /**
     * @notice Parses the gas used from the revert msg
     * @param _returnData the return data of requiredTxGas
     */
    function parseGasUsed(bytes memory _returnData) internal pure returns (uint256) {
        // If the _res length is less than 68, then the transaction failed silently (without a revert message)
        if (_returnData.length < 68) return 0; //"Transaction reverted silently";

        assembly {
            // Slice the sighash.
            _returnData := add(_returnData, 0x04)
        }
        return stringToUint(abi.decode(_returnData, (string))); // All that remains is the revert string
    }
}
