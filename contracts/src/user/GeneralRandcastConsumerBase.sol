// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "../utils/RequestIdBase.sol";
import "../utils/GasEstimationBase.sol";
import "./BasicRandcastConsumerBase.sol";
import "openzeppelin-contracts/contracts/access/Ownable.sol";

/**
 * @notice This provides callbackGaslimit auto-calculating and TODO balance checking to save user's effort.
 */
abstract contract GeneralRandcastConsumerBase is
    BasicRandcastConsumerBase,
    RequestIdBase,
    GasEstimationBase,
    Ownable
{
    // Sets user seed as 0 to so that users don't have to pass it.
    uint256 private constant USER_SEED_PLACEHOLDER = 0;
    // Default blocks the working group to wait before responding to the request.
    uint16 private constant DEFAULT_REQUEST_CONFIRMATIONS = 6;
    // TODO Gives a fixed buffer so that some logic differ in the callback slightly raising gas used will be supported.
    uint256 private constant GAS_FOR_CALLBACK_OVERHEAD = 30_000;
    // Dummy randomness for estimating gas of callback.
    uint256 private constant RANDOMNESS_PLACEHOLDER =
        103921425973949831153159651530394295952228049817797655588722524414385831936256;
    // Auto-calculating CallbackGasLimit in the first request call, also user can set it manually.
    uint256 public callbackGasLimit;
    // Auto-estimating CallbackMaxGasFee as 3 times tx.gasprice of the request call, also user can set it manually.
    // notes: tx.gasprice stands for effective_gas_price even post EIP-1559
    // priority_fee_per_gas = min(transaction.max_priority_fee_per_gas, transaction.max_fee_per_gas - block.base_fee_per_gas)
    // effective_gas_price = priority_fee_per_gas + block.base_fee_per_gas
    uint256 public callbackMaxGasFee;

    function setCallbackGasConfig(
        uint256 _callbackGasLimit,
        uint256 _callbackMaxGasFee
    ) external onlyOwner {
        callbackGasLimit = _callbackGasLimit;
        callbackMaxGasFee = _callbackMaxGasFee;
    }

    function requestRandomness(RequestType requestType, bytes memory params)
        internal
        calculateCallbackGasLimit
        returns (bytes32)
    {
        uint256 rawSeed = makeRandcastInputSeed(
            USER_SEED_PLACEHOLDER,
            msg.sender,
            nonce
        );
        nonce = nonce + 1;
        // This should be identical to controller generated requestId.
        bytes32 requestId = makeRequestId(rawSeed);
        // Only in the first place we calculate the callbackGasLimit, then next time we directly use it to request randomness.
        if (callbackGasLimit == 0) {
            // Prepares the message call of callback function according to request type
            bytes memory data;
            if (requestType == RequestType.Randomness) {
                data = abi.encodeWithSelector(
                    this.rawFulfillRandomness.selector,
                    requestId,
                    RANDOMNESS_PLACEHOLDER
                );
            } else if (requestType == RequestType.RandomWords) {
                uint32 numWords = abi.decode(params, (uint32));
                uint256[] memory randomWords = new uint256[](numWords);
                for (uint256 i = 0; i < numWords; i++) {
                    randomWords[i] = uint256(
                        keccak256(abi.encode(RANDOMNESS_PLACEHOLDER, i))
                    );
                }
                data = abi.encodeWithSelector(
                    this.rawFulfillRandomWords.selector,
                    requestId,
                    randomWords
                );
            } else if (requestType == RequestType.Shuffling) {
                uint32 upper = abi.decode(params, (uint32));
                uint256[] memory arr = new uint256[](upper);
                for (uint256 k = 0; k < upper; k++) {
                    arr[k] = k;
                }
                data = abi.encodeWithSelector(
                    this.rawFulfillShuffledArray.selector,
                    requestId,
                    arr
                );
            }

            // We don't want message call for estimating gas to take effect, therefore success should be false,
            // and result should be the reverted reason, which in fact is gas used we encoded to string.
            (bool success, bytes memory result) = address(this).call(
                abi.encodeWithSelector(
                    this.requiredTxGas.selector,
                    address(this),
                    0,
                    data
                )
            );

            // This will be 0 if message call for callback fails,
            // we pass this message to tell user that callback implementation need to be checked.
            uint256 gasUsed = parseGasUsed(result);

            require(
                !success && gasUsed != 0,
                "Fail to execute fulfillRandomness callback on dry-run, please check the code."
            );

            callbackGasLimit = gasUsed + GAS_FOR_CALLBACK_OVERHEAD;
        }
        return
            rawRequestRandomness(
                requestType,
                params,
                IAdapter(controller).getLastSubscription(address(this)),
                USER_SEED_PLACEHOLDER,
                DEFAULT_REQUEST_CONFIRMATIONS,
                callbackGasLimit,
                callbackMaxGasFee == 0 ? tx.gasprice * 3 : callbackMaxGasFee
            );
    }
}
