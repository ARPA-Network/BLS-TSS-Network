# Randcast Solidity Contracts

## Usage

```bash
cd contracts

forge install # Install submodule dependencies
forge build # Compile contracts
forge test --gas-price 1000000000 # Run Tests

# Run a specific test
forge test --match-test CommitDkg --gas-price 1000000000 -vvvvv
```

## Coverage

Measure coverage by installing the vscode extension: [coverage gutters](https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters)

```bash
forge coverage --report lcov
```
