# Randcast OP Automation Script

## Working OP Repo.

```bash
git clone https://github.com/wrinkledeth/optimism 
# changes made in this repo can be found below

# reset to latest stable build
git reset --hard f30fdd3f2a25c87c57531f0a33dbf3d902a7ca57

# edit bedrock-devnet/devnet 
['docker-compose', 'build', '--progress', 'plain'] # replace this with
["docker", "compose", "build", "--progress", "plain"] # this

# Makefile
docker-compose # replace this with
docker compose # this

# dockerfiles
# (search for string $BUILDPLATFORM) and add this to top:
ARG BUILDPLATFORM=linux/amd64
```


## Start Commands

```bash
git submodule update --init
make devnet-up-deploy 
make devnet-clean
```

## Troubleshoot Batcher

```bash
# error
ubuntu@jammy:~/optimism$ docker logs f6804639456b
t=2023-08-11T19:46:55+0000 lvl=info msg=“Initializing Batch Submitter”
t=2023-08-11T19:46:55+0000 lvl=eror msg=“Unable to create Batch Submitter” error=“querying rollup config: Post \“http://op-node:8545\“: dial tcp 172.18.0.4:8545: connect: connection refused”
t=2023-08-11T19:46:55+0000 lvl=crit msg=“Application failed”               message=“querying rollup config: Post \“http://op-node:8545\“: dial tcp 172.18.0.4:8545: connect: connection refused”

# just restart the container
docker start be5377ffaac6  ## OR

# in path: optimism/ops-bedrock
cd ops-bedrock && docker compose start op-batcher && cd ..
```

----

# Script automation Dev

## Ruoshan modifications

```bash
ubuntu@jammy:~/BLS-TSS-Network$ git status
On branch script_automation
Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
	modified:   contracts/.env.example
	modified:   contracts/README.md
	modified:   contracts/script/ControllerLocalTest.s.sol

Untracked files:
  (use "git add <file>..." to include in what will be committed)
	contracts/script/OPControllerOracleInitializationLocalTest.s.sol
	contracts/script/OPControllerOracleLocalTest.s.sol
	contracts/src/ControllerOracle.sol
	contracts/src/ControllerRelayer.sol
	contracts/src/OPChainMessenger.sol
	contracts/src/interfaces/IChainMessenger.sol
	contracts/src/interfaces/IControllerOracle.sol
	contracts/src/interfaces/IOPCrossDomainMessenger.sol
	contracts/test/ControllerOracle.t.sol
	contracts/test/MockL2CrossDomainMessenger.sol
```

## Scripting Steps

1. Initialize devnet (L1 + L2)
2. Deploy L2 randcast contracts
3. 
4. Deploy L1 randcast contracts.
5. Deploy 3 randcast nodes with docker compose and wait for grouping.
6. (Test request randomness on L1)
7. (Test request randomness on L2)

```bash
# prep env file + dependencies
cd BLS-TSS_NETWORK/contracts
cp .env.example .env
forge test # download dependencies

# deploy L1 contracts
forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://localhost:8545 --broadcast

forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://localhost:8545 --broadcast -g 150

## deploy L2 contracts
forge script script/OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript --fork-url http://localhost:9545 --broadcast

forge script script/OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript --fork-url http://localhost:9545 --broadcast

# start randcast nodes
cd BLS-TSS_NETWORK/scripts
docker-compose up -d

# 

```

```bash
# L2: OP Controller Oracle Local Test (update the L2 contract addresses in .env)
forge script script/OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript --fork-url http://localhost:9545 --broadcast

# L1 Controller Local Test (update the L1 contract addresses in .env)
forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://localhost:8545 --broadcast

# OP Controller Oracle Initialization
forge script script/OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript --fork-url http://localhost:9545 --broadcast

# StakeNode Local test script
forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://localhost:8545 --broadcast -g 150
```


## Dockercompose commands

```bash
docker-compose up -d
docker-compose down
```



## Tutorial: Communication between contracts on OP Mainnet and Ethereum

[crosshain calls tutorial](https://github.com/ethereum-optimism/optimism-tutorial/tree/main/cross-dom-comm)

```bash
export GOERLI_URL=http://localhost:8545
export OP_GOERLI_URL=http://localhost:9545

export GREETER_L1=0x4d0fcc1Bedd933dA4121240C2955c3Ceb68AAE84
export GREETER_L2=0xE8B462EEF7Cbd4C855Ea4B65De65a5c5Bab650A9




# deploy l1 to l2 contract:
export FROM_L1_CONTROLLER=0x13cac39046b6f66d544476b8a4FF72f884c56833

# deploy l2 to l2 contract:
export FROM_L2_CONTROLLER=0x13cac39046b6f66d544476b8a4FF72f884c56833

# tx hash:
export HASH=0x504b801ec9af5ca50b8521592dfa7b2bcd97ff9490304512122f05f2a49c9bd7

