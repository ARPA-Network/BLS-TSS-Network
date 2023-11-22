// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IOPGasPriceOracle} from "../interfaces/IOPGasPriceOracle.sol";

library ChainHelper {
    address public constant OP_GAS_PRICE_ORACLE_ADDR = address(0x420000000000000000000000000000000000000F);
    uint256 public constant OP_MAINNET_CHAIN_ID = 10;
    uint256 public constant OP_GOERLI_TESTNET_CHAIN_ID = 420;
    uint256 public constant OP_DEVNET_L1_CHAIN_ID = 900;
    uint256 public constant OP_DEVNET_L2_CHAIN_ID = 901;
    uint256 public constant BASE_MAINNET_CHAIN_ID = 8453;
    uint256 public constant BASE_GOERLI_TESTNET_CHAIN_ID = 84531;

    uint32 public constant OP_BASIC_FULFILLMENT_L1_GAS_USED = 5016;
    uint32 public constant OP_FULFILLMENT_GAS_PER_PARTICIPANT = 652;
    uint256 public constant OP_DIVISOR_DECIMALS = 6;

    function getBlockTime() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_GOERLI_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_GOERLI_TESTNET_CHAIN_ID
        ) {
            return 2;
        } else if (chainId == OP_DEVNET_L1_CHAIN_ID) {
            return 3;
        }
        return 12;
    }

    function getCurrentTxL1GasFees() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_GOERLI_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_GOERLI_TESTNET_CHAIN_ID
        ) {
            return IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).getL1Fee(msg.data);
        }
        return 0;
    }

    function getTxL1GasFees(uint256 l1GasUsed) public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_GOERLI_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_GOERLI_TESTNET_CHAIN_ID
        ) {
            uint256 l1Fee = l1GasUsed * IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).l1BaseFee();
            uint256 divisor = 10 ** OP_DIVISOR_DECIMALS;
            uint256 unscaled = l1Fee * IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).scalar();
            return unscaled / divisor;
        }
        return 0;
    }

    function getFulfillmentTxL1GasUsed(uint32 groupSize) public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_GOERLI_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_GOERLI_TESTNET_CHAIN_ID
        ) {
            return OP_BASIC_FULFILLMENT_L1_GAS_USED + groupSize * OP_FULFILLMENT_GAS_PER_PARTICIPANT;
        }
        return 0;
    }
}
