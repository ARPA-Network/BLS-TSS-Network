# Randcast Solidity Contracts

## Building and Testing

NOTE: foundry.toml must contain "gas_price = 1000000000" for tests to pass

```bash
cd contracts

forge install # Install submodule dependencies
forge build # Compile contracts

forge test # Run tests
forge test --match-test CommitDkg -vvvvv # Run a specific test
```

## Coverage

Measure coverage by installing the vscode extension: [coverage gutters](https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters)

```bash
forge coverage --report lcov
```

## Useful forge shell aliases

```bash
alias fw="forge test --watch --run-all"
alias ft="forge test --watch -vvv --match-test"
alias fc="forge coverage --report lcov"
```

## Scenario Testing Notes

[DKGScenarioTests.md](./DKGScenarioTests.md): DKG Scenarios

[ExtendedScenarioTests.md](./ExtendedScenarioTests.md): Rebalancing and Grouping Scenarios
