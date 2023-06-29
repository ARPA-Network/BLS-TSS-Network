- [Arpa Node](#arpa-node)
- [Node Client](#node-client)
  - [Usage](#usage)
- [ARPA Node CLI](#arpa-node-cli)
  - [Usage](#usage-1)
    - [REPL Commands](#repl-commands)
- [Management grpc server](#management-grpc-server)
- [Dependencies](#dependencies)
- [Troubleshooting](#troubleshooting)
- [Node Config](#node-config)
- [Local Test](#local-test)

# Arpa Node

This crate provides a node implementation to the DKG and Threshold-BLS based on-chain randomness service(Randcast).

The Arpa Node consists of an event queue, two types of task schedulers and a set of listeners and subscribers. Components are event-driving. All the components and data access layer(with sqlite) are wrapped in a context, which holds and shares all the information needed to expose services.

Note that task schedulers manage fixed-handlers and sub-handlers created by listeners, subscribers and grpc servers as different task types, which differ from DKG or BLS tasks the network publishes.

# Node Client

Node Client is a long-running program to run the ARPA node.

If the data path in the config file doesn't exist, as the first time to run the node, the client will generate a DKG keypair(served as the identity during a grouping process), then register the node address with dkg public key to the ARPA Network on-chain. In early access, make sure the address of the node has been added to eligible operators list in the Staking contract with sufficient stake in advance.

## Usage

```bash
cargo run --bin node-client
```

To print help, use `-- -h`:

```bash
cargo run --bin node-client -- -h
```

To specify a config file, use `-- -c <config_file>`:

```bash
cargo run --bin node-client -- -c conf/config.yml
```

# ARPA Node CLI

ARPA Node CLI is a fast and verbose REPL for the operator of a ARPA node. The same node config file as Node Client will be used. As a supplement to Node Client, it provides a set of commands to inspect the node status and interact with the on-chain contracts, e.g. register node to the network manually when error occurs in the node client.

## Usage

To print help, use `-- -h`:

```bash
cargo run --bin node-shell -- -h
```

To specify a config file, use `-- -c <config_file>`:

```bash
cargo run --bin node-shell -- -c conf/config.yml
```

To set the history file path, use `-- -H <history_file>`:

```bash
cargo run --bin node-shell -- -H node-shell.history
```

### REPL Commands

```text
Commands:
  show      Show information of the config file and node database
  call      Get views from on-chain contracts
  history   Show command history
  send      *** Be careful this will change on-chain state and cost gas ***
                Send trxs to on-chain contracts
  generate  Generate node identity(wallet) corresponding to ARPA node format
  inspect   Connect to the node client and inspect the node status
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

# Management grpc server

Management grpc server supports inspecting states and interacting with a running node.

Please see [`management.proto`](proto/management.proto) for detailed apis.

# Dependencies

Install [protoc](https://github.com/hyperium/tonic#dependencies) and [foundry](https://github.com/foundry-rs/foundry#installation), then run

```bash
cargo build
```

# Troubleshooting

"error: linker `cc` not found"... when running `cargo build`

```bash
sudo apt install build-essential
sudo apt install pkg-config
sudo apt install libssh-dev
```

# Node Config

Configuration items in [`conf/config.yml`](conf/config.yml) are listed here:

- node_committer_rpc_endpoint: Endpoint that this node will use to create server socket to expose committer grpc services. Once this get changed, the node MUST re-activate itself to the controller so that the controller can update the endpoint by re-grouping. (example: "0.0.0.0:50060")

- node_advertised_committer_rpc_endpoint: Endpoint that other members in the group will use to connect to this node. If this setting is not set, then value of node_committer_rpc_endpoint will be used here and published to other nodes. Note: This setting is updated every time the node starts, but it will not be broadcasted to other nodes until next re-grouping. (example: "10.0.0.1:50060")

- node_management_rpc_endpoint: Config endpoint to expose management grpc services. (example: "0.0.0.0:50099")

- node_management_rpc_token: Config token phrase for authenticaing management grpc requests by `authorization` header. (example: "arpa_network")

- provider_endpoint: Config endpoint to interact with chain provider. (example: "http://127.0.0.1:8545")

- chain_id: Config chain id of main chain. (example: 31337)

- controller_address: Config on-chain arpa network controller contract address. (example: "0x0000000000000000000000000000000000000001")

- data_path(Optional): Config DB file for persistence. (example: "data.sqlite")

- logger(Optional): Config logger settings.

  - example(default):

    ```
    logger:
      node_id: running
      context_logging: false
      log_file_path: log/running/
      rolling_file_size: 10 gb
    ```

  - node_id: Set a node id for logging.
  - context_logging: Set whether to log context of current node info and group info. Since it will increase log size, it is recommended to set it to false in production.
  - log_file_path: Set log file path. The `node-client` will create a `node.log` as well as a `node_err.log` under `log_file_path`, then log to them with info level and error level respectively.
  - rolling_file_size: Log file will be deleted when it reaches this size limit. The following units are supported (case insensitive):
    "b", "kb", "kib", "mb", "mib", "gb", "gib", "tb", "tib". The unit defaults to bytes if not specified.

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
        path: "m/44'/60'/0'/0"
        index: 0
        passphrase: "custom_password"
    ```

    Path and passphrase are optional.

    To protect secrets, several items can be set with literal `env` as placeholder. Their env keys are:

  - ARPA_NODE_MANAGEMENT_SERVER_TOKEN (node_management_rpc_token)
  - ARPA_NODE_ACCOUNT_PRIVATE_KEY (account, private_key)
  - ARPA_NODE_ACCOUNT_KEYSTORE_PASSWORD (account, keystore, password)
  - ARPA_NODE_HD_ACCOUNT_MNEMONIC (account, hdwallet, mnemonic)

- time_limits(Optional): Config time limits for different tasks. All the time limits are in milliseconds or block numbers.

  - example:
    ```
    time_limits:
      dkg_timeout_duration: 40
      randomness_task_exclusive_window: 10
      listener_interval_millis: 10000
      dkg_wait_for_phase_interval_millis: 10000
      provider_polling_interval_millis: 10000
      contract_transaction_retry_descriptor:
        base: 2
        factor: 1000
        max_attempts: 3
        use_jitter: true
      contract_view_retry_descriptor:
        base: 2
        factor: 500
        max_attempts: 5
        use_jitter: true
      commit_partial_signature_retry_descriptor:
        base: 2
        factor: 1000
        max_attempts: 5
        use_jitter: false
    ```
  - These values need to be set according to config of on-chain Controller contract.

    - dkg_timeout_duration: Block numbers between DKG start and timeout. (example: 40)
    - randomness_task_exclusive_window: Block numbers when a randomness task can be only fulfilled by the assigned group. (example: 10)

  - These values can be set by node owner or administrator according to the rate limitation of the provider. Setting a small value would be to node's advantage in responding tasks. It's recommended to set a value no larger than the block time of the chain.

    - listener_interval_millis: Milliseconds between two rounds of listeners. (example: 10000)
    - dkg_wait_for_phase_interval_millis: Milliseconds between two rounds of polling for the next DKG phase. (example: 10000)
    - provider_polling_interval_millis: Milliseconds between two rounds of polling events from provider. (example: 10000)

  - We use exponential backoff to retry when an interaction fails. The interval will be an exponent of base multiplied by factor every time. The interval will be reset when the interaction succeeds.

  - A jitter is added to the interval to avoid the situation that all the tasks are polling at the same time. It will multiply a random number between 0.5 and 1.0 to the interval.

    - contract_transaction_retry_descriptor: (interval sequence without jitter: 2s, 4s, 8s)
    - contract_view_retry_descriptor: (interval sequence without jitter: 1s, 2s, 4s, 8s, 16s)
    - commit_partial_signature_retry_descriptor: (interval sequence without jitter: 2s, 4s, 8s, 16s, 32s)

- listeners(Optional): Config listeners to run with node client to customize services. By default all the listeners will be enabled. All of them can be disabled by setting an empty value explicitly.

  - example:

  ```
  listeners:
    - l_type: Block
      interval_millis: 0
      use_jitter: true
    - l_type: NewRandomnessTask
      interval_millis: 0
      use_jitter: true
    - l_type: PreGrouping
      interval_millis: 0
      use_jitter: true
    - l_type: PostCommitGrouping
      interval_millis: 10000
      use_jitter: true
    - l_type: PostGrouping
      interval_millis: 10000
      use_jitter: true
    - l_type: ReadyToHandleRandomnessTask
      interval_millis: 10000
      use_jitter: true
    - l_type: RandomnessSignatureAggregation
      interval_millis: 2000
      use_jitter: false
  ```

  - Block, NewRandomnessTask, PreGrouping, PostCommitGrouping, PostGrouping, ReadyToHandleRandomnessTask, RandomnessSignatureAggregation are the types of listeners. We use a fixed interval to retry when a listen round fails. The interval_millis and use_jitter are the same as the time_limits.

    - The polling intervals of Block, NewRandomnessTask and PreGrouping are decided by provider_polling_interval_millis in time_limits.

    - The polling of PostCommitGrouping, PostGrouping, ReadyToHandleRandomnessTask are triggered by view calls on the chain, so the interval_millis should be set to a value no larger than the block time of the chain.

    - The polling of RandomnessSignatureAggregation is triggered by the node itself, so the interval_millis can be set relatively small.

# Local Test

```bash
# unit tests
cargo test --all -- --test-threads=1 --nocapture
```

Start the local testnet by anvil:

```bash
# produces a new block every 1 second
anvil --block-time 1
```

Deploy the Controller and the Adapter contract:

```bash
cd contracts
# controller address 0xdc64a140aa3e981100a9beca4e685f962f0cf6c9
# adapter_address: 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853
# user contract address 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e
forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://localhost:8545 --broadcast
```

Add operators, start the Staking pool and stake for a user and some nodes:

```bash
# nodes addresses are generated from index 10 by mnemonic "test test test test test test test test test test test junk"(anvil default)
# offset and length can be set by STAKING_NODES_INDEX_OFFSET and STAKING_NODES_INDEX_LENGTH in .env
forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://localhost:8545 --broadcast -g 150
```

Run 3 nodes to make a group:

```bash
cd crates/arpa-node
cargo run --bin node-client -- -c test/conf/config_test_1.yml
cargo run --bin node-client -- -c test/conf/config_test_2.yml
cargo run --bin node-client -- -c test/conf/config_test_3.yml
```

Deploy the user contract([`GetRandomNumberExample`](../../contracts/src/user/examples/GetRandomNumberExample.sol)) and request a randomness:

```bash
cd contracts
# this should be executed after we have an available group as logging e.g."Group index:0 epoch:1 is available, committers saved." in node terminal
forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://localhost:8545 --broadcast
```

The nodes should sign the randomness and one of the committers in the group will fulfill the result, check the results by `cast`:

```bash
# check the randomness result recorded by the adapter and the user contract respectively
cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 \
  "getLastRandomness()(uint256)"

cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e \
  "lastRandomnessResult()(uint256)"

# the above two outputs of uint256 type should be identical
```
