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
