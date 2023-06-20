
# Create EC2 Instance in new VPC with Systems Manager enabled

This example includes:

- Own VPC with public subnet (following AWS Defaults for new accounts)
- Based on latest Amazon Linux 2
- System Manager replaces SSH (Remote session available trough the AWS Console or the AWS CLI.)
- Userdata executed from script in S3 (configure_anvil.sh, configure_node.sh)

## Useful commands

- `cdk bootstrap`   initialize assets before deploy
- `cdk synth`       emits the synthesized CloudFormation template
- `cdk deploy`      deploy this stack to your default AWS account/region
- `aws ssm start-session --target i-xxxxxxxxx` remote session for shell access

## Install AWS CLI and AWS Session Manager plugin

[install aws cli](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)

[install session manager plugin](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html)

```bash
# Configure aws-cli
aws configure # configure aws cli (us-east-2, json, none, none)

# Verify session manager plugin
session-manager-plugin
```

## Install CDK and deploy

[cdk getting started](https://docs.aws.amazon.com/cdk/v2/guide/getting_started.html)

[working with cdk python](https://docs.aws.amazon.com/cdk/v2/guide/work-with-cdk-python.html)

```bash
# Install CDK
npm install -g aws-cdk 
cdk --version # check version

# activate venv and install requirements.
cd ec2_cdk
python3 -m venv .venv 
source .venv/bin/activate
pip install -r requirements.txt

# Deploying
cdk bootstrap # initialize assets before deploy
cdk synth # emits the synthesized CloudFormation template
cdk deploy # deploy this stack to your default AWS account/region

# destory
cdk destroy # destroy this stack to your default AWS account/region
```

## Sample workflow

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
# ssh anvil-test

The anvil container should start automatically. If not, consider the following options. 

# monitor anvil 
watch "docker logs anvil-chain | tail -n 20"

# pull and run anvil image
docker pull wrinkledeth/anvil-chain:latest
docker run -d --name anvil-chain -p 8545:8545 wrinkledeth/anvil-chain:latest
watch "docker logs anvil-chain | tail -n 20"

# run anvil without docker
anvil --host 0.0.0.0 --block-time 1 --prune-history

-----------------
################
# ssh node-test

# prep env
export ETH_RPC_URL="http://18.218.143.46:8545"
export NODE_RPC_IP="3.15.228.9"
cd /tmp
git clone -b dockerAutomation https://github.com/wrinkledeth/BLS-TSS-Network.git

# run container-init
docker run -d --name contract-init -v /tmp/BLS-TSS-Network/contracts/.env:/usr/src/app/external/.env -e ETH_RPC_URL=$ETH_RPC_URL wrinkledeth/contract-init:latest
# monitor logs
watch "docker logs contract-init | tail -n 20" 

# run arpa-nodes
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v /tmp/BLS-TSS-Network/docker/mainnet/arpa-node/config_1.yml:/usr/src/app/external/config.yml -e ETH_RPC_URL=$ETH_RPC_URL -e NODE_RPC_URL=${NODE_RPC_IP}:50061 wrinkledeth/arpa-node:latest
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

-----------------

```

## Useful Commands

```bash

The node-test ec2 comes with a dockerized version of foundry. You can run foundry commands using it like this:
[docs](https://book.getfoundry.sh/tutorials/foundry-docker)

```bash
# run cast using foundry docker image
docker run foundry "cast block --rpc-url $RPC_URL latest"

# Cleanup Docker images
docker kill $(docker ps -q)
docker rm $(docker ps -a -q)
```
