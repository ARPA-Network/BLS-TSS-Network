// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

/**
 * @dev This function generates a shuffled array of length equal to the given 'upper' limit.
 * @param upper The upper limit on the range of numbers in the array.
 * @param randomness The initial seed for the random number generator used in shuffling.
 * @return arr Returns a randomly shuffled array
 */
function shuffle(uint256 upper, uint256 randomness) pure returns (uint256[] memory) {
    uint256[] memory arr = new uint256[](upper);
    for (uint256 k = 0; k < upper; k++) {
        arr[k] = k;
    }
    uint256 i = arr.length;
    uint256 j;
    uint256 t;

    while (--i > 0) {
        j = randomness % i;
        randomness = uint256(keccak256(abi.encode(randomness)));
        t = arr[i];
        arr[i] = arr[j];
        arr[j] = t;
    }

    return arr;
}

/**
 * @dev This function takes a seed, an array of indices, and the number of indices to choose.
 * It returns an array of randomly chosen indices.
 * @param seed The seed used for random number generation.
 * @param indices The array of indices from which to choose.
 * @param choosenCount The number of indices to choose.
 * @return An array of randomly chosen indices.
 */
function draw(uint256 seed, uint256[] memory indices, uint256 choosenCount) pure returns (uint256[] memory) {
    uint256[] memory chosenIndices = new uint256[](choosenCount);

    // Create copy of indices to avoid modifying original array.
    uint256[] memory remainingIndices = new uint256[](indices.length);
    for (uint256 i = 0; i < indices.length; i++) {
        remainingIndices[i] = indices[i];
    }

    uint256 remainingCount = remainingIndices.length;
    for (uint256 i = 0; i < choosenCount; i++) {
        uint256 index = uint256(keccak256(abi.encodePacked(seed, i))) % remainingCount;
        chosenIndices[i] = remainingIndices[index];
        remainingIndices[index] = remainingIndices[remainingCount - 1];
        remainingCount--;
    }
    return chosenIndices;
}

/**
 * @dev This function performs a simple random roll to pick an index.
 * @param randomness Random number used in determining the chosen index.
 * @param size The range (or size) of possible indices.
 * @return chosenIndex The randomly chosen index.
 */
function roll(uint256 randomness, uint256 size) pure returns (uint256 chosenIndex) {
    return randomness % size;
}

/**
 * @dev This function use the random number picks a index based on an array of weights.
 * @param randomness The random number used to choose the index.
 * @param valueWeights An array of weight values corresponding to indices.
 * @return chosenIndex The picked index.
 */
function pickByWeights(uint256 randomness, uint256[] memory valueWeights) pure returns(uint256 chosenIndex) {
    uint256 weightsLen = valueWeights.length;
    uint256 sum = 0;
    if (weightsLen > 1) {
        for (uint256 i = 1; i < weightsLen; ++i) {
            valueWeights[i] = valueWeights[i] + valueWeights[i - 1];
        }
    }
    sum = valueWeights[weightsLen - 1];
    uint256 weightValue = randomness % (sum + 1);
    for (uint256 i = 0; i < weightsLen; ++i) {
        if (weightValue <= valueWeights[i]) {
            return i;
        }
    }
}

/**
 * @dev This function generates a batch of random numbers based on a seed and a specified length.
 * @param seed The initial value used to generate randomness, also known as the seed.
 * @param length The number of random values you want to generate.
 * @return An array of random uint256 numbers.
 */
function batch(uint256 seed, uint256 length) pure returns (uint256[] memory) {
    uint256[] memory batchRandomness = new uint256[](length);
    for (uint256 i = 0; i < length; ++i) {
        batchRandomness[i] = uint256(keccak256(abi.encode(seed, i)));
    }
    return batchRandomness;
}
