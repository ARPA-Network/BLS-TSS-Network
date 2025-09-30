// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IOPGasPriceOracle} from "../interfaces/IOPGasPriceOracle.sol";
import {ArbSys} from "./arb/ArbSys.sol";
import {ArbGasInfo} from "./arb/ArbGasInfo.sol";

library ChainHelper {
    uint32 public constant BASIC_FULFILLMENT_L1_GAS_USED = 5016;
    uint32 public constant FULFILLMENT_GAS_PER_PARTICIPANT = 652;
    uint256 public constant DECIMALS = 6;
    uint256 public constant BLOCK_TIME_DENOMINATOR = 1000;

    uint256 private constant ETHEREUM_MAINNET_CHAIN_ID = 1;
    uint256 private constant BSC_MAINNET_CHAIN_ID = 56;
    uint256 private constant BSC_TESTNET_CHAIN_ID = 97;

    // Optimism
    address private constant OP_GAS_PRICE_ORACLE_ADDR = address(0x420000000000000000000000000000000000000F);

    uint256 private constant OP_MAINNET_CHAIN_ID = 10;
    uint256 private constant OP_SEPOLIA_TESTNET_CHAIN_ID = 11155420;
    uint256 private constant OP_DEVNET_L1_CHAIN_ID = 900;
    uint256 private constant OP_DEVNET_L2_CHAIN_ID = 901;
    uint256 private constant BASE_MAINNET_CHAIN_ID = 8453;
    uint256 private constant BASE_SEPOLIA_TESTNET_CHAIN_ID = 84532;
    uint256 private constant REDSTONE_MAINNET_CHAIN_ID = 690;
    uint256 private constant REDSTONE_GARNET_TESTNET_CHAIN_ID = 17069;
    uint256 private constant REDSTONE_HOLESKY_TESTNET_CHAIN_ID = 17001;
    uint256 private constant LOOT_MAINNET_CHAIN_ID = 5151706;
    uint256 private constant LOOT_GOERLI_TESTNET_CHAIN_ID = 9088912;
    uint256 private constant TAIKO_KATLA_TEST_CHAIN_ID = 167008;
    uint256 private constant B3_MAINNET_CHAIN_ID = 8333;
    uint256 private constant B3_TESTNET_CHAIN_ID = 1993;

    // Arbitrum
    address private constant ARBSYS_ADDR = address(0x0000000000000000000000000000000000000064);
    ArbSys private constant ARBSYS = ArbSys(ARBSYS_ADDR);

    address private constant ARBGAS_ADDR = address(0x000000000000000000000000000000000000006C);
    ArbGasInfo private constant ARBGAS = ArbGasInfo(ARBGAS_ADDR);

    uint256 private constant ARB_MAINNET_CHAIN_ID = 42161;
    uint256 private constant ARB_GOERLI_TESTNET_CHAIN_ID = 421613;
    uint256 private constant ARB_SEPOLIA_TESTNET_CHAIN_ID = 421614;
    uint256 private constant ARPACHAIN_MAINNET_CHAIN_ID = 4224;

    function getRequestExpirationBlockNumberDuration() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == ARB_MAINNET_CHAIN_ID || chainId == ARB_GOERLI_TESTNET_CHAIN_ID
                || chainId == ARB_SEPOLIA_TESTNET_CHAIN_ID
        ) {
            return 1 days * BLOCK_TIME_DENOMINATOR / 250;
        } else if (chainId == ARPACHAIN_MAINNET_CHAIN_ID) {
            return 3600;
        }
        return 1 days * BLOCK_TIME_DENOMINATOR / getBlockTime();
    }

    function getBlockTime() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (
            chainId == OP_MAINNET_CHAIN_ID || chainId == OP_SEPOLIA_TESTNET_CHAIN_ID || chainId == OP_DEVNET_L2_CHAIN_ID
                || chainId == BASE_MAINNET_CHAIN_ID || chainId == BASE_SEPOLIA_TESTNET_CHAIN_ID
                || chainId == REDSTONE_HOLESKY_TESTNET_CHAIN_ID || chainId == REDSTONE_MAINNET_CHAIN_ID
                || chainId == REDSTONE_GARNET_TESTNET_CHAIN_ID
        ) {
            return 2 * BLOCK_TIME_DENOMINATOR;
        } else if (chainId == OP_DEVNET_L1_CHAIN_ID || chainId == TAIKO_KATLA_TEST_CHAIN_ID) {
            return 3 * BLOCK_TIME_DENOMINATOR;
        } else if (chainId == LOOT_MAINNET_CHAIN_ID || chainId == LOOT_GOERLI_TESTNET_CHAIN_ID) {
            return 5 * BLOCK_TIME_DENOMINATOR;
        } else if (chainId == B3_MAINNET_CHAIN_ID || chainId == B3_TESTNET_CHAIN_ID) {
            return 1 * BLOCK_TIME_DENOMINATOR;
        } else if (chainId == BSC_MAINNET_CHAIN_ID) {
            return 750;
        } else if (chainId == BSC_TESTNET_CHAIN_ID) {
            return 750;
        }
        return 12 * BLOCK_TIME_DENOMINATOR;
    }

    function getCurrentTxL1GasFees() public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (_isArbitrumChainId(chainId)) {
            return ARBGAS.getCurrentTxL1GasFees();
        } else if (_isOPChainId(chainId)) {
            return IOPGasPriceOracle(OP_GAS_PRICE_ORACLE_ADDR).getL1Fee(msg.data);
        }
        return 0;
    }

    function getTxL1GasFees(uint256 l1GasUsed) public view returns (uint256) {
        uint256 chainId = block.chainid;
        if (_isArbitrumChainId(chainId)) {
            (, uint256 l1PricePerByte,,,,) = ARBGAS.getPricesInWei();
            // see https://developer.arbitrum.io/devs-how-tos/how-to-estimate-gas#where-do-we-get-all-this-information-from
            // for the justification behind the 140 number.
            return l1PricePerByte * (l1GasUsed / 9 + 140);
        } else if (_isOPChainId(chainId)) {
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
        if (_isOPChainId(chainId) || _isArbitrumChainId(chainId)) {
            return BASIC_FULFILLMENT_L1_GAS_USED + groupSize * FULFILLMENT_GAS_PER_PARTICIPANT;
        }
        return 0;
    }

    function _getBlockNumber() internal view returns (uint256) {
        uint256 chainid = block.chainid;
        if (_isArbitrumChainId(chainid)) {
            return ARBSYS.arbBlockNumber();
        }
        return block.number;
    }

    function _isArbitrumChainId(uint256 chainId) internal pure returns (bool) {
        return chainId == ARB_MAINNET_CHAIN_ID || chainId == ARB_GOERLI_TESTNET_CHAIN_ID
            || chainId == ARB_SEPOLIA_TESTNET_CHAIN_ID || chainId == ARPACHAIN_MAINNET_CHAIN_ID;
    }

    function _isOPChainId(uint256 chainId) internal pure returns (bool) {
        return chainId == OP_MAINNET_CHAIN_ID || chainId == OP_SEPOLIA_TESTNET_CHAIN_ID
            || chainId == OP_DEVNET_L2_CHAIN_ID || chainId == BASE_MAINNET_CHAIN_ID
            || chainId == BASE_SEPOLIA_TESTNET_CHAIN_ID || chainId == REDSTONE_MAINNET_CHAIN_ID
            || chainId == REDSTONE_GARNET_TESTNET_CHAIN_ID || chainId == B3_MAINNET_CHAIN_ID
            || chainId == B3_TESTNET_CHAIN_ID;
    }
}
