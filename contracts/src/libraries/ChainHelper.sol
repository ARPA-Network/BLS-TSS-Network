// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IOPGasPriceOracle} from "../interfaces/IOPGasPriceOracle.sol";

library ChainHelper {
    address public constant OP_GAS_PRICE_ORACLE_ADDR = address(0x420000000000000000000000000000000000000F);
    uint256 public constant OP_MAINNET_CHAIN_ID = 10;
    uint256 public constant OP_SEPOLIA_TESTNET_CHAIN_ID = 11155420;
    uint256 public constant OP_DEVNET_L1_CHAIN_ID = 900;
    uint256 public constant OP_DEVNET_L2_CHAIN_ID = 901;
    uint256 public constant BASE_MAINNET_CHAIN_ID = 8453;
    uint256 public constant BASE_SEPOLIA_TESTNET_CHAIN_ID = 84532;
    uint256 public constant REDSTONE_MAINNET_CHAIN_ID = 690;
    uint256 public constant REDSTONE_GARNET_TESTNET_CHAIN_ID = 17069;
    uint256 public constant REDSTONE_HOLESKY_TESTNET_CHAIN_ID = 17001;
    uint256 public constant LOOT_MAINNET_CHAIN_ID = 5151706;
    uint256 public constant LOOT_GOERLI_TESTNET_CHAIN_ID = 9088912;
    uint256 public constant TAIKO_KATLA_TEST_CHAIN_ID = 167008;

    uint32 public constant BASIC_FULFILLMENT_L1_GAS_USED = 5016;
    uint32 public constant FULFILLMENT_GAS_PER_PARTICIPANT = 652;
    uint256 public constant DECIMALS = 6;

    function getBlockTime() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_SEPOLIA_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_SEPOLIA_TESTNET_CHAIN_ID
                || chainId == REDSTONE_HOLESKY_TESTNET_CHAIN_ID || chainId == REDSTONE_MAINNET_CHAIN_ID
                || chainId == REDSTONE_GARNET_TESTNET_CHAIN_ID
        ) {
            return 2;
        } else if (chainId == OP_DEVNET_L1_CHAIN_ID || chainId == TAIKO_KATLA_TEST_CHAIN_ID) {
            return 3;
        } else if (chainId == LOOT_MAINNET_CHAIN_ID) {
            return 5;
        } else if (chainId == LOOT_GOERLI_TESTNET_CHAIN_ID) {
            return 200;
        }
        return 12;
    }

    function getCurrentTxL1GasFees() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_SEPOLIA_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_SEPOLIA_TESTNET_CHAIN_ID
                || chainId == REDSTONE_MAINNET_CHAIN_ID || chainId == REDSTONE_GARNET_TESTNET_CHAIN_ID
        ) {
            return IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).getL1Fee(msg.data);
        }
        return 0;
    }

    function getTxL1GasFees(uint256 l1GasUsed) public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_SEPOLIA_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_SEPOLIA_TESTNET_CHAIN_ID 
                || chainId == REDSTONE_MAINNET_CHAIN_ID || chainId == REDSTONE_GARNET_TESTNET_CHAIN_ID
        ) {
            try IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).isEcotone() returns (bool isEcotone) {
                if (isEcotone) {
                    uint256 scaledBaseFee = IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).baseFeeScalar() * 16
                        * IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).l1BaseFee();
                    uint256 scaledBlobBaseFee = IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).blobBaseFeeScalar()
                        * IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).blobBaseFee();
                    uint256 fee = l1GasUsed * (scaledBaseFee + scaledBlobBaseFee);
                    return fee / (16 * 10 ** DECIMALS);
                }
            } catch {
                uint256 l1Fee = l1GasUsed * IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).l1BaseFee();
                uint256 divisor = 10 ** DECIMALS;
                uint256 unscaled = l1Fee * IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).scalar();
                return unscaled / divisor;
            }
        }
        return 0;
    }

    function getFulfillmentTxL1GasUsed(uint32 groupSize) public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_SEPOLIA_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_SEPOLIA_TESTNET_CHAIN_ID
                || chainId == REDSTONE_MAINNET_CHAIN_ID || chainId == REDSTONE_GARNET_TESTNET_CHAIN_ID
        ) {
            return BASIC_FULFILLMENT_L1_GAS_USED + groupSize * FULFILLMENT_GAS_PER_PARTICIPANT;
        }
        return 0;
    }
}
