# Arpa Node

This crate provides a node side on-chain implementation to the provided DKG and Threshold-BLS based randomness service(Randcast).

The Arpa Node consists of an event queue, two types of task schedulers and a set of listeners and subscribers. Events are passed within components and drive them to work. All the components and data access layer(with sqlite) are wrapped in a context, which holds and shares all the information needed for the client bin and grpc servers to expose services.

Note that task schedulers manage components and sub-handlers from listeners, subscribers and grpc servers as different task types, instead of DKG or BLS tasks the network publishes.

# Node-client bin

Node-client is a long-running client to run the ARPA node.

With structopt, now it is more explicit and self-explanatory:

```bash
cargo run --bin node-client -- -h
```

# Management grpc server

Management grpc server supports getting states and interacting with a running node. It can be used for scenario tests or DevOps.

Please see [`management.proto`](proto/management.proto) for detailed apis.

# Node-account-client bin(WIP)

Node-account-client is a practical tool to generate keystore corresponding to ARPA node format.

# Node-cmd-client bin(WIP)

Node-cmd-client is a practical tool to interact with on-chain contracts for ARPA node owner or administrator.

# Dependencies

Install [protoc](https://github.com/hyperium/tonic#dependencies) and [foundry](https://github.com/foundry-rs/foundry#installation), then run

```bash
cargo build
```

# Node Config

Configuration items in [`conf/config.yml`](conf/config.yml) are listed here:

- node_committer_rpc_endpoint: Config endpoint to expose committer grpc services. (example: "[::1]:50060")

- node_management_rpc_endpoint: Config endpoint to expose management grpc services. (example: "[::1]:50099")

- node_management_rpc_token: Config token phrase for authenticaing management grpc requests by `authorization` header. (example: "arpa_network")

- provider_endpoint: Config endpoint to interact with chain provider. (example: "http://127.0.0.1:8545")

- chain_id: Config chain id of main chain. (example: 31337)

- controller_address: Config on-chain arpa network controller contract address. (example: "0x0000000000000000000000000000000000000001")

- data_path(Optional): Config DB file for persistence. (example: "data.sqlite")

- account: Config node identity in the network. There are three available account types.
  - example(not recommended): private_key: "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
  - example:
    ```
    keystore:
        password: env
        path: test.keystore
    ```
  - example:
    ```
    hdwallet:
        mnemonic: env
        path(Optional): "m/44'/60'/0'/0"
        index: 1
        passphrase(Optional): "custom_password"
    ```
- listeners(Optional): Config listeners to run with node client to customize services. By default all the listeners will be enabled. All of them can be disabled by setting a empty value explicitly.
  - example:
  ```
  - Block
  - PreGrouping
  - PostCommitGrouping
  - PostGrouping
  - NewRandomnessTask
  - ReadyToHandleRandomnessTask
  - RandomnessSignatureAggregation
  ```

To protect secrets, several items can be set with literal `env` as placeholder. Their env keys are:

- ARPA_NODE_MANAGEMENT_SERVER_TOKEN (node_management_rpc_token)
- ARPA_NODE_ACCOUNT_PRIVATE_KEY (account, private_key)
- ARPA_NODE_ACCOUNT_KEYSTORE_PASSWORD (account, keystore, password)
- ARPA_NODE_HD_ACCOUNT_MNEMONIC (account, hdwallet, mnemonic)

# Local Test

## start the local testnet by anvil:

```bash
# produces a new block every 1 second and ignores contract size for now
# evm errors can be reduced by increasing block-time properly
anvil --block-time 1
```

## deploy the controller and the adapter contract:

```bash
cd contracts
# controller address 0x5fc8d32690cc91d4c39d9d3abcbd16989f875707
# adapter_address: 0x0165878a594ca255338adfa4d48449f69242eb8f
# user contract address 0x8464135c8f25da09e49bc8782676a84730c318bc
forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://localhost:8545 --optimize --broadcast --slow
```

### add operators, start the staking pool and stake for a user and some nodes:

```bash
# nodes addresses are generated from index 10 by mnemonic "test test test test test test test test test test test junk"(anvil default)
# offset and length can be set by STAKING_NODES_INDEX_OFFSET and STAKING_NODES_INDEX_LENGTH in .env
forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://localhost:8545 --optimize --broadcast --slow
```

## run 3 nodes to make a group:

```bash
cd crates/arpa-node
cargo run --bin node-client -- -m new-run -c conf/config_test_1.yml
cargo run --bin node-client -- -m new-run -c conf/config_test_2.yml
cargo run --bin node-client -- -m new-run -c conf/config_test_3.yml
```

## deploy the user contract([`GetRandomNumberExample`](../../contracts/src/user/examples/GetRandomNumberExample.sol)) and request a randomness:

```bash
cd contracts
# this should be executed after we have an available group as logging e.g."Group index:0 epoch:1 is available, committers saved." in node terminal
forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://localhost:8545 --broadcast --slow
```

## the nodes should sign the randomness and one of the committers in the group will fulfill the result

## check the results by cast:

```bash
# check the randomness result recorded by the adapter and the user contract respectively
cast call 0x0165878a594ca255338adfa4d48449f69242eb8f \
  "getLastRandomness()(uint256)"

cast call 0x8464135c8f25da09e49bc8782676a84730c318bc \
  "lastRandomnessResult()(uint256)"

# the above two outputs of uint256 type should be identical
```
