// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

interface IOPGasPriceOracle {
    /// @notice Computes the L1 portion of the fee based on the size of the rlp encoded input
    ///         transaction, the current L1 base fee, and the various dynamic parameters.
    /// @param _data Unsigned fully RLP-encoded transaction to get the L1 fee for.
    /// @return L1 fee that should be paid for the tx
    function getL1Fee(bytes memory _data) external view returns (uint256);
    /// @notice Retrieves the latest known L1 base fee.
    /// @return Latest known L1 base fee.
    function l1BaseFee() external view returns (uint256);
    /// @notice Retrieves the current blob base fee.
    /// @return Current blob base fee.
    function blobBaseFee() external view returns (uint256);
    /// @notice Retrieves the current base fee scalar.
    /// @return Current base fee scalar.
    function baseFeeScalar() external view returns (uint32);
    /// @notice Retrieves the current blob base fee scalar.
    /// @return Current blob base fee scalar.
    function blobBaseFeeScalar() external view returns (uint32);
    /// @notice Indicates whether the network has gone through the Ecotone upgrade.
    /// @return True if the network has gone through the Ecotone upgrade, false otherwise.
    function isEcotone() external view returns (bool);
    /// @notice Retrieves the current fee scalar. This is deprecated and will be removed in future.
    /// @return Current fee scalar.
    function scalar() external view returns (uint256);
}
