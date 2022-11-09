# Arpa Node

This crate provides a node side implementation as well as a demo to the provided DKG and Threshold-BLS based randomness service(Randcast).

# Node-client bin

Node-client is a long-running client to run the ARPA node.

With structopt, now it is more explicit and self-explanatory:

```bash
cargo run --bin node-client -- -h
```

# Node-account-client bin(WIP)

Node-account-client is a practical tool to generate keystore corresponding to ARPA node format.

# Node-cmd-client bin(WIP)

Node-cmd-client is a practical tool to interact with on-chain contracts for ARPA node owner or administrator.

# User-client bin(WIP)

User-client is a practical tool to interact with on-chain contracts for Randcast users.

Note: Basically for demo use, in real environment a Randcast user should request and receive randomness by extending consumer contract instead of calling controller contract through an EOA directly.

# Dependencies

Install [protoc](https://github.com/hyperium/tonic#dependencies) and [foundry](https://github.com/foundry-rs/foundry#installation), then run

```bash
cargo build
```

# Demo Steps

## deploy contract server(different ip endpoints on different chains):

```bash
cargo run --bin controller-server "[::1]:50052"
cargo run --bin adapter-server "[::1]:50053"
```

## run nodes:

```bash
cd crates/arpa-node
cargo run --bin node-client -- -m demo -i 1
cargo run --bin node-client -- -m demo -i 2
cargo run --bin node-client -- -m demo -i 3
```

```bash
cargo run --bin node-client -- -m demo -i 4
cargo run --bin node-client -- -m demo -i 5
cargo run --bin node-client -- -m demo -i 6
```

## use user-client to request randomness:

```bash
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" request foo
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50053" request bar
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" last_output
```

## use node-cmd-client to get views or call some helper methods(1 - controller 2 - adapter):

```bash
cargo run --bin node-cmd-client 0x90000000000000000000000000000000000000ad "[::1]:50052" "1" set_initial_group "[::1]:50053"
cargo run --bin node-cmd-client 0x9000000000000000000000000000000000000001 "[::1]:50052" "1" get_group "1"
cargo run --bin node-cmd-client 0x9000000000000000000000000000000000000001 "[::1]:50053" "2" get_group "1"
```

## 1 MainChain Demo(Happy Path) Example:

```bash
# deploy contract
cargo run --bin controller-server "[::1]:50052"
```

```bash
# run 3 nodes to prepare a BLS-ready group
cd crates/arpa-node
cargo run --bin node-client -- -m demo -i 1
cargo run --bin node-client -- -m demo -i 2
cargo run --bin node-client -- -m demo -i 3
```

```bash
# check result by view get_group
cargo run --bin node-cmd-client 0x9000000000000000000000000000000000000001 "[::1]:50052" "2" get_group "1"
```

```bash
# now we can request randomness task as a user on main chain
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" request foo
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" last_output
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" request bar
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" last_output
# verify result by view last_output and node logs
```

## 1 MainChain + n AdapterChain Demo(Happy Path, n=1 here) Example:

```bash
# deploy contract
cargo run --bin controller-server "[::1]:50052"
cargo run --bin adapter-server "[::1]:50053"
```

```bash
# run 3 nodes to prepare the first BLS-ready group
cd crates/arpa-node
cargo run --bin node-client -- -m demo -i 1
cargo run --bin node-client -- -m demo -i 2
cargo run --bin node-client -- -m demo -i 3
```

```bash
# relay the first BLS-ready group to adapter chain by authenticated admin manully
cargo run --bin node-cmd-client 0x90000000000000000000000000000000000000ad "[::1]:50052" "1" set_initial_group "[::1]:50053"
# check result by view get_group
cargo run --bin node-cmd-client 0x9000000000000000000000000000000000000001 "[::1]:50053" "2" get_group "1"
```

```bash
# run 3 nodes to prepare the second BLS-ready group
cargo run --bin node-client -- -m demo -i 4
cargo run --bin node-client -- -m demo -i 5
cargo run --bin node-client -- -m demo -i 6
# then the second BLS-ready group will be relayed by the first BLS-ready group to adapter chain
# check result by view get_group
cargo run --bin node-cmd-client 0x9000000000000000000000000000000000000001 "[::1]:50053" "2" get_group "2"
```

```bash
# now we can request randomness task as a user on main/adapter chain
# the tasks will be sent to group 1 and group 2 in turn
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" request foo
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" last_output
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" request bar
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50052" last_output

cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50053" request foo
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50053" last_output
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50053" request foo
cargo run --bin user-client 0x9000000000000000000000000000000000000001 "[::1]:50053" last_output
# verify result by view last_output and node logs
```
