<h1 align="center">Randcast BLS-TSS-Network</h1>

## Overview

The infrastructure of Randcast(Threshold-BLS based randomness service).

## Build Guide

Build with `cargo build (--release)`.

Test with `cargo test --all -- --test-threads=1 --nocapture`.

## Directory Structure

This repository contains several Rust crates that implement the different building blocks of Randcast. The high-level structure of the repository is as follows:

- [`arpa-node`](crates/arpa-node): Rust crate that provides a Node CLI and an off-chain demo to the provided DKG and Threshold-BLS based randomness service(Randcast).
- [`randcast-contract-mock`](crates/randcast-contract-mock): Rust crate that provides a mock implementation to the contracts of Randcast.
- [`dkg-core`](crates/dkg-core): Rust crate that provides the implementation utilities for the DKG
- [`threshold-bls`](crates/threshold-bls): threshold BLS signatures for BN254 and BLS12-381

## Acknowledgements

Rust crates [`dkg-core`](crates/dkg-core), [`threshold-bls`](crates/threshold-bls) and Solidity contracts under [libraries/](contracts/src/libraries) have been adapted from the following resources:

- [celo-threshold-bls-rs](https://github.com/celo-org/celo-threshold-bls-rs)
- [bls_solidity_python](https://github.com/ChihChengLiang/bls_solidity_python)

## Disclaimers

**This software has not been audited. Use at your own risk.**
