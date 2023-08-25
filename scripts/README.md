# ARPA Network L1/L2 Devnet Automation

## Usage

Start Optimism Devnet

```bash
cd optimism
make devnet-up-deploy
```

Build node client (CHECK THIS)

```bash
cd crates/arpa-node
cargo build --bin node-client
``` 

Deploy ARPA Network contracts to L1 and L2 and start randcast nodes

```bash
cd scripts
# activate venv
python3 -m venv .venv
source .venv/bin/activate
# install dependencies
pip3 install -r requirements.txt
# run script
python3 main.py
```

Kill Existing randcast nodes

```bash
# containers
docker kill $(docker ps -q -f ancestxor=arpachainio/node:latest); docker rm -f $(docker ps -a -q -f ancestor=arpachainio/node:latest)

# processes
pkill -f 'node-client -c'
rm -rf /home/ubuntu/BLS-TSS-Network/crates/arpa-node/log
rm /home/ubuntu/BLS-TSS-Network/crates/arpa-node/*.sqlite

# delete the node logs
rm -rf /home/ubuntu/BLS-TSS-Network/crates/arpa-node/log
```

Clean and redploy OP devnet

```bash
cd optimism
make devnet-clean
make devnet-up-deploy
```

## View Calls

Request randomness:

```bash
# L1
forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://localhost:8545 --broadcast
# L2
forge script script/OPGetRandomNumberLocalTest.s.sol:OPGetRandomNumberLocalTestScript --fork-url http://localhost:9545 --broadcast 
```

Some view calls:

```bash
# Get latest l1 group info
cast call <Controller> "getGroup(uint256)" 0 --rpc-url http://127.0.0.1:8545
# Check if the latest group info is relayed to L2
cast call <ControllerOracle> "getGroup(uint256)" 0 --rpc-url http://127.0.0.1:9545
# Check if the randomness is successfully fulfilled on L1
cast call <L1_ERC1962Proxy> "getLastRandomness()(uint256)" --rpc-url http://127.0.0.1:8545
# check if the randomness is successfully fulfilled on L2
cast call <L2_ERC1962Proxy> "getLastRandomness()(uint256)" --rpc-url http://127.0.0.1:9545
```

```bash
################ with contract addresses ################
# Get latest l1 group info
cast call 0x9d4454B023096f34B160D6B654540c56A1F81688 "getGroup(uint256)" 0 --rpc-url http://127.0.0.1:8545
# Check if the latest group info is relayed to L2
cast call 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 "getGroup(uint256)" 0 --rpc-url http://127.0.0.1:9545
# Check if the randomness is successfully fulfilled on L1
cast call 0x809d550fca64d94Bd9F66E60752A544199cfAC3D "getLastRandomness()(uint256)" --rpc-url http://127.0.0.1:8545
# Check if the randomness is successfully fulfilled on L2
cast call 0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9 "getLastRandomness()(uint256)" --rpc-url http://127.0.0.1:9545
``` 