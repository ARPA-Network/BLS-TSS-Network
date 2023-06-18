# Randcast Solidity Contracts

## Dependencies

Install [foundry](https://github.com/foundry-rs/foundry#installation).

## Building and Testing

NOTE: foundry.toml must contain "gas_price = 1000000000" for tests to pass

```bash
cd contracts

forge install # Install submodule dependencies
forge build # Compile contracts

forge test # Run tests
forge test --mt CommitDkg -vvvvv # Run a specific test
```

## Coverage

Measure coverage by installing the vscode extension: [coverage gutters](https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters)

```bash
forge coverage --report lcov
```

## Useful forge shell aliases

```bash
- alias fw="forge test --watch --run-all"
- alias ft="forge test --watch -vvv --match-test"
- alias fc="forge coverage --report lcov"
```

## Scenario Testing Notes

[DKGScenarioTests.md](./docs/DKGScenarioTests.md): DKG Scenarios

[ExtendedScenarioTests.md](./docs/ExtendedScenarioTests.md): Rebalancing and Grouping Scenarios

## Slither Statica Analysis

Static analysis has been conducted on our smart contracts to ensure that they are secure and free of bugs. In addition, we offer a tool to help analyze slitehr output. Details can be found here: [SlitherStaticAnalysis.md](./slither/SlitherStaticAnalysis.md)

---

## Local Test

Create a `.env` following the `.env.example`, then add `ADMIN_PRIVATE_KEY`, `USER_PRIVATE_KEY` and `STAKING_NODES_MNEMONIC` to it or set them in the environment.

### start the local testnet by anvil:

```bash
# automatically generates a new block as soon as a transaction is submitted
anvil
```

### deploy the staking, the controller and the adapter contract:

```bash
# see .env for more deployment addresses
forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://localhost:8545 --broadcast
```

### add operators, start the staking pool and stake for a user and some nodes:

```bash
# nodes addresses are generated from index 10 by mnemonic "test test test test test test test test test test test junk"(anvil default)
# offset and length can be set by STAKING_NODES_INDEX_OFFSET and STAKING_NODES_INDEX_LENGTH in .env
forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://localhost:8545 --broadcast -g 150
```

### run some nodes to get an available group:

See crate arpa-node [`README.md`](../crates/arpa-node/README.md) for details.

### deploy the user contract([`GetRandomNumberExample`](src/user/examples/GetRandomNumberExample.sol)) and request a randomness:

```bash
# this should be executed after we have an available group as logging e.g."Group index:0 epoch:1 is available, committers saved." in node terminal
forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://localhost:8545 --broadcast
```

### use cast to call views or send transactions to contracts we deployed:

e.g.

```bash
cast call [contract_deployment_address] [function_signature] [function_input_params]
cast send [contract_deployment_address] [function_signature] [function_input_params] --private-key [sender_private_key]
cast rpc [rpc_method_name] [rpc_method_input_params]
cast receipt [transaction_hash]
```

