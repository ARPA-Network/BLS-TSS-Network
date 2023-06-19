# ARPA Node Docker Documentation

## Links

[Official Rust Image](https://www.docker.com/blog/simplify-your-deployments-using-the-rust-official-image/)

[Foundry Image](https://github.com/foundry-rs/foundry/pkgs/container/foundry)

[Codebase](https://github.com/ARPA-Network/BLS-TSS-Network)

## Backlog

- [ ] Kick off container creation with github actions.
- [ ] setup anvil chain ec2 instance
  - [ ] setup proper networking
- [ ] Create interactive cli tool for generaeting yml files / docker run commands.

## To do

- [ ] Allow node containers to connect to host network
- [ ] open the rpc endpoint ports on the docker container (port range?)

## Anvil Commands

[anvil docs](https://book.getfoundry.sh/reference/anvil/)

interesting anvil containers
[eulith](https://github.com/Eulith/eulith-in-a-box/blob/master/start.sh)
[keulith docker](https://hub.docker.com/layers/keulith/devrpc/latest/images/sha256-763b225dff8c52cacb05e8fbfd3357bacb830c086d1802cc80790806a7d7dfab?context=explore)
[docker-anvil](https://github.com/hananbeer/docker-anvil/blob/main/Dockerfile)
[fork-chain](https://github.com/zekiblue/fork-chain/tree/master)
[cannon](https://github.com/usecannon/cannon)

```bash

anvil --block-time 10 --prune-history --silent &

--prune-history # don't keep full chain history
--silent # don't print logs to stdout
--block-time 1 # block time interval in seconds
--no-mining # disable interval mining, mine on demand.

# If you run into disk space issues, these commands can help debug
# find the offending logs
du -aBM --max-depth 1 | sort -nr | head -10
# sample log location
/root/.foundry/anvil/tmp/anvil-state-11-06-2023-03-35qDAeUy
```

## Docker commands

```bash
docker ls # list containers
docker inspect <container_id> # inspect container

docker network ls # list networks
docker inspect <network_id> # inspect network
docker network connect <network_id> <container_id> # attach container to network

docker image ls # list images
```

---

## Local Testnet using Docker Network (no ports exposed externally)

[docker networking](https://docs.docker.com/network/)

```bash

##############
## LOCALNET ##
##############

# create network
docker network create randcast_network 

# build iamges
cd BLS-TSS-Network
docker build -t anvil-chain ./docker/localnet/anvil-chain
docker build -t contract-init -f ./docker/localnet/contract-init/Dockerfile .
docker build -t arpa-node ./docker/localnet/arpa-node


# Start anvil chain
docker run -d --network randcast_network --name anvil-chain anvil-chain:latest

# Run contract init (ensure .env configured correctly)
docker run -d --network randcast_network --name contract-init -v ./contracts/.env:/usr/src/app/external/.env contract-init:latest 
# Wait for all contracts to deploy (check docker logs)

# Run 3 arpa nodes (ensure config files are correct)
docker run -d --network randcast_network --name node1 -v ./docker/localnet/arpa-node/config_1.yml:/usr/src/app/external/config.yml arpa-node:latest 
docker run -d --network randcast_network --name node2 -v ./docker/localnet/arpa-node/config_2.yml:/usr/src/app/external/config.yml arpa-node:latest 
docker run -d --network randcast_network --name node3 -v ./docker/localnet/arpa-node/config_3.yml:/usr/src/app/external/config.yml arpa-node:latest 

# check if nodes grouped succesfully 
# (exec into node1 container)
watch 'cat /usr/src/app/log/1/node.log | grep "available"'
  # "Group index:0 epoch:1 is available, committers saved."

# deploy user contract
# (exec into contract-init container)
forge script /usr/src/app/script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://anvil-chain:8545 --broadcast

# check the randomness result recorded by the adapter and the user contract respectively
export ETH_RPC_URL=http://anvil-chain:8545

cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 "getLastRandomness()(uint256)"

cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e "lastRandomnessResult()(uint256)"

# the above two outputs of uint256 type should be identical
```

## Mainnet Node Operatoration (publishing ports externally on host)

```bash
#############
## MAINNET ##
#############

# build images
cd BLS-TSS-Network
docker build -t anvil-chain ./docker/mainnet/anvil-chain
docker build -t contract-init -f ./docker/mainnet/contract-init/Dockerfile .
docker build -t arpa-node ./docker/mainnet/arpa-node


# Start anvil chain
docker run -d --name anvil-chain -p 8545:8545 anvil-chain:latest

# set ETH RPC ENDPOINT (IP:PORT) and NODE_IP (IP ONLY)
export RPC_ENDPOINT="1.1.1.1:8545"
export NODE_IP="1.2.3.4"

# Run contract init (ensure .env configured correctly)
docker run -d --name contract-init -v ./contracts/.env:/usr/src/app/external/.env -e RPC_ENDPOINT=$RPC_ENDPOINT contract-init:latest

# Run 3 arpa nodes (ensure config files are correct)
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v ./docker/mainnet/arpa-node/config_1.yml:/usr/src/app/external/config.yml -e RPC_ENDPOINT=$RPC_ENDPOINT -e NODE_ENDPOINT=${NODE_IP}:50091 arpa-node:latest
docker run -d --name node2 -p 50061:50061 -p 50092:50091 -v ./docker/mainnet/arpa-node/config_2.yml:/usr/src/app/external/config.yml -e RPC_ENDPOINT=$RPC_ENDPOINT -e NODE_ENDPOINT=${NODE_IP}:50092 arpa-node:latest
docker run -d --name node3 -p 50061:50061 -p 50093:50091 -v ./docker/mainnet/arpa-node/config_3.yml:/usr/src/app/external/config.yml -e RPC_ENDPOINT=$RPC_ENDPOINT -e NODE_ENDPOINT=${NODE_IP}:50093 arpa-node:latest

# check if nodes grouped succesfully 
# (exec into node1 container)
watch 'cat /usr/src/app/log/1/node.log | grep "available"'
  # "Group index:0 epoch:1 is available, committers saved."

# deploy user contract
# (exec into contract-init container)
forge script /usr/src/app/script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://anvil-chain:8545 --broadcast

# check the randomness result recorded by the adapter and the user contract respectively
export ETH_RPC_URL=http://${RPC_ENDPOINT}
cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 "getLastRandomness()(uint256)"
cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e "lastRandomnessResult()(uint256)"

# the above two outputs of uint256 type should be identical
```

## Publish images to docker hub

[publish your own image](https://docs.docker.com/get-started/publish-your-own-image/)
[arpa node cargo build memory issues](https://github.com/rust-lang/cargo/issues/10781#issuecomment-1163819998)
[rust nightly builds](https://hub.docker.com/r/rustlang/rust/tags)
[working solution](https://yaleman.org/post/2022/2022-10-23-docker-rust-cargo-and-137-errors/)

```bash
## Force M1 macs to build for linux/amd64
export DOCKER_DEFAULT_PLATFORM=linux/amd64

# build images
cd BLS-TSS-Network
docker build -t anvil-chain ./docker/mainnet/anvil-chain
docker build -t contract-init -f ./docker/mainnet/contract-init/Dockerfile .
docker build -t arpa-node ./docker/mainnet/arpa-node

# tag images
docker tag anvil-chain:latest wrinkledeth/anvil-chain:latest
docker tag contract-init:latest wrinkledeth/contract-init:latest
docker tag arpa-node:latest wrinkledeth/arpa-node:latest

# go into docker desktop and push images to repo

# pull images from repo
docker pull wrinkledeth/anvil-chain:latest
docker pull wrinkledeth/contract-init:latest
docker pull wrinkledeth/arpa-node:latest
```

## Todo

- [x] Figure out how to copy container-init files from outside contracts folder
- [ ] Organize docker folders (localnet and mainnet)
- [ ] Test publish commands, write docs
- [ ] Test one ec2, write docs

## AWS EC2 Config

```bash
# Anvil-test ec2
3.16.69.78
ssh ec2-user@3.16.69.78 -i ~/.ssh/arpa_aws_keypair.pem 

# Node-test ec2
3.128.204.225
ssh ec2-user@3.128.204.225 -i ~/.ssh/arpa_aws_keypair.pem



```

## Anvil ec2

```bash
# anvil-test
ssh ec2-user@3.16.69.78 -i ~/.ssh/arpa_aws_keypair.pem # ssh into ec2

## install docker
sudo yum update -y
sudo yum -y install docker
sudo service docker start
sudo usermod -a -G docker ec2-user
sudo chmod 666 /var/run/docker.sock
sudo systemctl enable docker.service # enable docker on boot
sudo systemctl start docker.service # start docker
docker ps

# pull and run anvil image
docker pull wrinkledeth/anvil-chain:latest
docker run -d --name anvil-chain -p 8545:8545 wrinkledeth/anvil-chain:latest

# check for listening port 
netstat -tulpn | grep LISTEN
```

## Node EC2

```bash

## ssh to node
ssh ec2-user@3.128.204.225 -i ~/.ssh/arpa_aws_keypair.pem


# install docker
sudo yum update -y
sudo yum -y install docker git
sudo service docker start
sudo usermod -a -G docker ec2-user
sudo chmod 666 /var/run/docker.sock
sudo systemctl enable docker.service # enable docker on boot
sudo systemctl start docker.service # start docker
docker ps

# export env vars
export RPC_ENDPOINT="3.16.69.78:8545"
export NODE_IP="3.128.204.225"
export ETH_RPC_URL=http://${RPC_ENDPOINT}


# pull container-init and arpa-node images
docker pull wrinkledeth/contract-init:latest
docker pull wrinkledeth/arpa-node:latest

# clone repo
git clone -b dockerAutomation https://github.com/wrinkledeth/BLS-TSS-Network.git
cd BLS-TSS-Network

# run container-init
docker run -d --name contract-init -v /home/ec2-user/BLS-TSS-Network/contracts/.env:/usr/src/app/external/.env -e RPC_ENDPOINT=$RPC_ENDPOINT wrinkledeth/contract-init:latest

# run arpa-nodes
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v ./docker/mainnet/arpa-node/config_1.yml:/usr/src/app/external/config.yml -e RPC_ENDPOINT=$RPC_ENDPOINT -e NODE_ENDPOINT=${NODE_IP}:50091 wrinkledeth/arpa-node:latest
docker run -d --name node2 -p 50061:50061 -p 50092:50091 -v ./docker/mainnet/arpa-node/config_2.yml:/usr/src/app/external/config.yml -e RPC_ENDPOINT=$RPC_ENDPOINT -e NODE_ENDPOINT=${NODE_IP}:50092 wrinkledeth/arpa-node:latest
docker run -d --name node3 -p 50061:50061 -p 50093:50091 -v ./docker/mainnet/arpa-node/config_3.yml:/usr/src/app/external/config.yml -e RPC_ENDPOINT=$RPC_ENDPOINT -e NODE_ENDPOINT=${NODE_IP}:50093 wrinkledeth/arpa-node:latest

# check if nodes grouped succesfully
docker exec -it node1 bash
watch 'cat /usr/src/app/log/1/node.log | grep "available"'

# check if randomness worked.
cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 "getLastRandomness()(uint256)"
cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e "lastRandomnessResult()(uint256)"

```

## install foundry

```bash
## install foundry
curl -L https://foundry.paradigm.xyz | bash && \
    /home/ec2-user/.foundry/bin/foundryup
source /home/ec2-user/.bashrc

git clone -b dockerAutomation https://github.com/wrinkledeth/BLS-TSS-Network.git
cd BLS-TSS-Network/contracts
forge test 
```
