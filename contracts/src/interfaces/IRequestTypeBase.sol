// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IRequestTypeBase {
    enum RequestType {
        Randomness,
        RandomWords,
        Shuffling
    }
}
