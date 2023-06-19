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

You will need to edit the following locations:

./docker/mainnet/arpa-node/config_X.yml
  node_committer_rpc_endpoint: "0.0.0.0:50061"
  node_advertised_committer_rpc_endpoint: "0.0.0.0:50061" # published port on host. See readme
  node_management_rpc_endpoint: "0.0.0.0:50091"


provider_endpoint: "http://0.0.0.0:8545" # would use alchemy or infura here

```bash

# build images
cd BLS-TSS-Network
docker build -t anvil-chain ./docker/mainnet/anvil-chain
docker build -t contract-init -f ./docker/mainnet/contract-init/Dockerfile .
docker build -t arpa-node ./docker/mainnet/arpa-node


# Start anvil chain
docker run -d --name anvil-chain -p 8545:8545 anvil-chain:latest
# 0.0.0.0:8545 should be accessible from outside 

# Run contract init (ensure .env configured correctly)
docker run -d --name contract-init -v ./contracts/.env:/usr/src/app/external/.env contract-init:latest

# Run 3 arpa nodes (ensure config files are correct)
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v ./docker/mainnet/arpa-node/config_1.yml:/usr/src/app/external/config.yml arpa-node:latest
docker run -d --name node2 -p 50062:50061 -p 50092:50091 -v ./docker/mainnet/arpa-node/config_2.yml:/usr/src/app/external/config.yml arpa-node:latest
docker run -d --name node3 -p 50063:50061 -p 50093:50091 -v ./docker/mainnet/arpa-node/config_3.yml:/usr/src/app/external/config.yml arpa-node:latest
# 0.0.0.0 : 50061, 50062, 50063 are the exposed rpc ports

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

## Todo

- [x] Figure out how to copy container-init files from outside contracts folder
- [ ] Organize docker folders (localnet and mainnet)
- [ ] Test publish commands, write docs
- [ ] Test one ec2, write docs

## Anvil ec2

```bash
# anvil-test
ssh ec2-user@3.16.69.78 -i ~/.ssh/arpa_aws_keypair.pem # ssh into ec2

## install docker
sudo yum update -y # update yum
sudo yum install docker git -y # install docker
sudo usermod -a -G docker ec2-user # add ec2-user to docker group
newgrp docker # log out and log back in
sudo systemctl enable docker.service # enable docker on boot
sudo systemctl start docker.service # start docker
sudo systemctl status docker.service # start docker
docker ps

## install foundry
curl -L https://foundry.paradigm.xyz | bash && \
    /home/ec2-user/.foundry/bin/foundryup
source /home/ec2-user/.bashrc

git clone -b dockerIntegration https://github.com/wrinkledeth/BLS-TSS-Network.git
cd BLS-TSS-Network/contracts
forge test
cd ..
docker build -t anvil-chain ./docker/mainnet/anvil-chain
docker build -t contract-init -f ./docker/mainnet/contract-init/Dockerfile .
docker build -t arpa-node ./docker/mainnet/arpa-node



```

## Node EC2

```bash
#!/bin/bash
sudo yum update -y
sudo yum -y install docker
sudo service docker start
sudo usermod -a -G docker ec2-user
sudo chmod 666 /var/run/docker.sock
```