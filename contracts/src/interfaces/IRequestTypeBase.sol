// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

interface IRequestTypeBase {
    enum RequestType {
        Randomness,
        RandomWords,
        Shuffling
    }
}
