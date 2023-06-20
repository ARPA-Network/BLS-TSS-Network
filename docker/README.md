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
3.21.113.65
ssh ec2-user@3.21.113.65 -i ~/.ssh/arpa_aws_keypair.pem
# 137 exit on forge test command.


```

## Anvil ec2

```bash
# anvil-test
ssh ec2-user@3.16.69.78 -i ~/.ssh/arpa_aws_keypair.pem # ssh into ec2

# Start anvil directly without docker
anvil --host 0.0.0.0 --block-time 1 --prune-history

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

# watch tail
watch "docker logs anvil-chain | tail"
```

## Node EC2

```bash

## ssh to node
ssh ec2-user@3.21.113.65 -i ~/.ssh/arpa_aws_keypair.pem


# install docker
sudo yum update -y
sudo yum -y install docker git
sudo service docker start
sudo usermod -a -G docker ec2-user
sudo chmod 666 /var/run/docker.sock
sudo systemctl enable docker.service # enable docker on boot
sudo systemctl start docker.service # start docker
docker ps

# clone repo
git clone -b dockerAutomation https://github.com/wrinkledeth/BLS-TSS-Network.git
cd BLS-TSS-Network

# pull container-init and arpa-node images
docker pull wrinkledeth/anvil-chain:latest
docker pull wrinkledeth/contract-init:latest
docker pull wrinkledeth/arpa-node:latest

# export env vars
export ETH_RPC_URL="http://3.16.69.78:8545" # anvil ec2 ip / alchemy rpc endpoint url
export NODE_RPC_IP="3.21.113.65" # node ec2 ip

# run container-init
docker run -d --name contract-init -v /home/ec2-user/BLS-TSS-Network/contracts/.env:/usr/src/app/external/.env -e ETH_RPC_URL=$ETH_RPC_URL wrinkledeth/contract-init:latest
watch "docker logs contract-init | tail"

# run arpa-nodes
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v /home/ec2-user/BLS-TSS-Network/docker/mainnet/arpa-node/config_1.yml:/usr/src/app/external/config.yml -e ETH_RPC_URL=$ETH_RPC_URL -e NODE_RPC_URL=${NODE_RPC_IP}:50091 wrinkledeth/arpa-node:latest
docker run -d --name node2 -p 50062:50061 -p 50092:50091 -v /home/ec2-user/BLS-TSS-Network/docker/mainnet/arpa-node/config_2.yml:/usr/src/app/external/config.yml -e ETH_RPC_URL=$ETH_RPC_URL -e NODE_RPC_URL=${NODE_RPC_IP}:50092 wrinkledeth/arpa-node:latest
docker run -d --name node3 -p 50063:50061 -p 50093:50091 -v /home/ec2-user/BLS-TSS-Network/docker/mainnet/arpa-node/config_3.yml:/usr/src/app/external/config.yml -e ETH_RPC_URL=$ETH_RPC_URL -e NODE_RPC_URL=${NODE_RPC_IP}:50093 wrinkledeth/arpa-node:latest

# check if nodes grouped succesfully
docker exec -it node1 /bin/bash
watch 'cat /usr/src/app/log/1/node.log | grep "available"'
cat /var/log/randcast_node_client.log

# deploy user contract / check if randomness worked.
docker exec -it contract-init /bin/bash
forge script /usr/src/app/script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url $ETH_RPC_URL --broadcast
cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 "getLastRandomness()(uint256)"
cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e "lastRandomnessResult()(uint256)"

# Troubleshooting... node1 randcast node_error.log
The errors in this log mainly involve two issues:

1. ContractClientError: The execution of a contract has been reverted, with the data "0x5291bbcf0000000000000000000000000000000000000000000000000000000000000000".

2. RpcResponseError with Unauthenticated status: The system is failing to send partial signatures to the committer addresses (0x71be63f3384f5fb98995898a86b02fb2426c5788 and 0xfabb0ac9d68b0b445fb7357272ff202c5651694a), as it is missing a valid authentication token. The errors are being retried several times but continue to occur. This issue causes further errors when trying to handle randomness tasks.

# nmap troubleshooting node rpc endpoints
nmap 3.21.113.65 -p 50091-50093  -Pn

# Cleanup
docker kill $(docker ps -q)
docker rm $(docker ps -a -q)
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

## Test Contract Addreses

```bash
Contract Address: 0x5FbDB2315678afecb367f032d93F642f64180aa3 # BLS.sol
Contract Address: 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 # GroupLib.sol
Contract Address: 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 # Arpa 
Contract Address: 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9 # Staking 
Contract Address: 0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9 # Controller
Contract Address: 0x0165878A594ca255338adfa4d48449f69242Eb8F # Adapter
Contract Address: 0xa513E6E4b8f2a923D98304ec87F64353C4D5C853 # Adapter Proxy

root@d4052e418329:/usr/src/app# cast call 0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9 "getGroup(uint256)" 0    
0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000000380000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000012ace53d435ec618e6f235796c6180bfa9690fecf733e559f5315fc5f1c495b292ee9f066514350a90d5aa9a99e52306a742262880ade09e1fee81e75f0cfd51e022a7bd671715be42aeacfc91a9b37980d30e60cac51f99411c4ca6abd4b479c2ecaa6fde07fc6a6e017ba8572c23fd8e80d14a45ddcefefeeba8c96540f0f900000000000000000000000000000000000000000000000000000000000000003000000000000000000000000bcd4042de499d14e55001ccbb24a551f3b95409606c12dc21d6da36e442e344beaad32705de52d74700cb907c74a7d1f1993b7400c6d8b73f3ae2c7db3b7cbbbc4477f391b2dd977dc4ef9a730dcbe5f8143eaf51cfff55ff981a594a322761e37bcc55914861900b2685eb7e4175e246741a669297ebe834dd8dee21749953b5f9216e5ae7d2c816e4fe388e9007d503dc04bfe00000000000000000000000071be63f3384f5fb98995898a86b02fb2426c578811646bbdfd6bcfe1300b617f963d185959f39c1f14f09534e7024c8fe02345c42c131c6adf9c5503d07f9da82f7b8defa64401971e9f09c8220824df55df664e0082ae98d02711ba3c89f7f817ec89f78246620e9e9daa3db78733a40400bbaf175461704e6278216f19b39bbcd3245ddba42b6c75f305e4cb2be87ea9201f12000000000000000000000000fabb0ac9d68b0b445fb7357272ff202c5651694a2aaebf5d3902735213375221a064dfb4883cf643a631354c06ec5a89bf686d7f0de4fdd2bde21fc22f77c6d7fc930c60bbdaa9f5ed51cb4893d678f6bf618a5e304c1329ba92dfe43cc0f18d69493e145b8c4fd7fce6865b1eb2414953a10e7e222064a85a882f1016a9780e43dccef3c05bf1fef070eebde949e4acd16fb5d40000000000000000000000000000000000000000000000000000000000000003000000000000000000000000fabb0ac9d68b0b445fb7357272ff202c5651694a00000000000000000000000071be63f3384f5fb98995898a86b02fb2426c5788000000000000000000000000bcd4042de499d14e55001ccbb24a551f3b95409600000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000000300000000000000000000000071be63f3384f5fb98995898a86b02fb2426c5788000000000000000000000000fabb0ac9d68b0b445fb7357272ff202c5651694a000000000000000000000000bcd4042de499d14e55001ccbb24a551f3b95409600000000000000000000000000000000000000000000000000000000000000012ace53d435ec618e6f235796c6180bfa9690fecf733e559f5315fc5f1c495b292ee9f066514350a90d5aa9a99e52306a742262880ade09e1fee81e75f0cfd51e022a7bd671715be42aeacfc91a9b37980d30e60cac51f99411c4ca6abd4b479c2ecaa6fde07fc6a6e017ba8572c23fd8e80d14a45ddcefefeeba8c96540f0f9000000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000000
```

## install cdk

[cdk getting started](https://docs.aws.amazon.com/cdk/v2/guide/getting_started.html)
[working with cdk python](https://docs.aws.amazon.com/cdk/v2/guide/work-with-cdk-python.html)

```bash
# Install AWS CLI
brew install awscli

# Install CDK
npm install -g aws-cdk 
cdk --version # check version

## Confgure AWS CLI
aws configure # configure aws cli (us-east-2, json, none, none)

# Create new CDK Project
mkdir ec2_cdk
cd ec2_cdk
cdk init app --language python # Initialize CDK Project

# activate venv and install requirements.
python3 -m venv .venv 
source .venv/bin/activate
pip install -r requirements.txt

# Deploying
npm install -g aws-cdk  # if not installed
cdk bootstrap # initialize assets before deploy
cdk synth
cdk deploy

# destory
cdk destroy
```

## Connecting to ec2

Make sure to install the AWS Session manager plugin first.

[install session manager plugin](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html)

```bash
# verify session manager installation
session-manager-plugin

# Connect to instance
aws ssm start-session --target i-059e25f9b598653c0

```

## Final commands for ec2

```bash

# Stack Outputs
Anvil ec2: 18.218.143.46
aws ssm start-session --target i-0b01c70aaed481b37

Node ec2: 3.15.228.9
aws ssm start-session --target i-091e4058778037871

Env variables for contract deployment:
export ETH_RPC_URL="http://18.218.143.46:8545"
export NODE_RPC_IP="3.15.228.9"
            

-----------------

################
# ssh anvil

The anvil container should start automatically. If not, consider the following options. 

# pull and run anvil image
docker pull wrinkledeth/anvil-chain:latest
docker run -d --name anvil-chain -p 8545:8545 wrinkledeth/anvil-chain:latest
watch "docker logs anvil-chain | tail -n 20"

# run anvil without docker
anvil --host 0.0.0.0 --block-time 1 --prune-history

-----------------
################
# ssh node


# run container-init
git clone -b dockerAutomation https://github.com/wrinkledeth/BLS-TSS-Network.git

docker run -d --name contract-init -v /tmp/BLS-TSS-Network/contracts/.env:/usr/src/app/external/.env -e ETH_RPC_URL=$ETH_RPC_URL wrinkledeth/contract-init:latest
watch "docker logs contract-init | tail -n 20"

# run arpa-nodes
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v /tm dp/BLS-TSS-Network/docker/mainnet/arpa-node/config_1.yml:/usr/src/app/external/config.yml -e ETH_RPC_URL=$ETH_RPC_URL -e NODE_RPC_URL=${NODE_RPC_IP}:50061 wrinkledeth/arpa-node:latest
docker run -d --name node2 -p 50062:50061 -p 50092:50091 -v /tmp/BLS-TSS-Network/docker/mainnet/arpa-node/config_2.yml:/usr/src/app/external/config.yml -e ETH_RPC_URL=$ETH_RPC_URL -e NODE_RPC_URL=${NODE_RPC_IP}:50062 wrinkledeth/arpa-node:latest
docker run -d --name node3 -p 50063:50061 -p 50093:50091 -v /tmp/BLS-TSS-Network/docker/mainnet/arpa-node/config_3.yml:/usr/src/app/external/config.yml -e ETH_RPC_URL=$ETH_RPC_URL -e NODE_RPC_URL=${NODE_RPC_IP}:50063 wrinkledeth/arpa-node:latest

# check if nodes grouped succesfully
docker exec -it node1 /bin/bash       
watch 'cat /usr/src/app/log/1/node.log | grep "available"'
cat /var/log/randcast_node_client.log

# deploy user contract / check if randomness worked.
docker exec -it contract-init /bin/bash
forge script /usr/src/app/script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url $ETH_RPC_URL --broadcast
cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 "getLastRandomness()(uint256)"
cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e "lastRandomnessResult()(uint256)"

# Cleanup
docker kill $(docker ps -q)
docker rm $(docker ps -a -q)

# install foundry
curl -L https://foundry.paradigm.xyz | bash && \
    /home/ssm-user/.foundry/bin/foundryup
source /home/ssm-user/.bashrc



```

forge script /tmp/BLS-TSS-Network/contracts/script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url $ETH_RPC_URL --broadcast

Todo:

- [ ] move mainnet cdk stuff into mainnet
- [ ] Create nodeoperator folder
- [ ] Make node operator cdk (stripped down)
- [ ] Make node operator docker image (takes config only)
- [ ] Write docs on how to use CDK + docker foundry + docker commands for node deployment. 