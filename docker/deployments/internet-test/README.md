
# Internet Enabled Testnet using CDK (AWS EC2 + docker)

This folder contains everything needed to deploy a fully functional testnet onling using two AWS EC2 instances.

The three folders in this directory each contain all files needed to build the corresponding docker image that are used in this deployment.

- anvil-chain: start a local anvil chain
- contract-init: deploy contracts to local anvil chain
- arpa-node: stand up an arpa node that interfaces with the deployed contracts to generate randomness

This example also includes a CDK script (located in ec2_cdk) which creates:

- 2 EC2 instances (anvil chain + contract deployment container /  node containers)
- A VPC with a public subnet
- Security Groups for each instance.
- Based on latest Amazon Linux 2 image.
- Systems Manager replaces SSH

Below are the instructions to test out the end to end flow of deploying randcast on docker containers running on AWS EC2 instances.

## Install AWS CLI and AWS Session Manager plugin

[install aws cli](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)

[install session manager plugin](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html)

```bash
# Configure aws-cli
aws configure # configure aws cli (us-east-2, json, none, none)

# Verify session manager plugin install
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


```

Sample Stack Outputs after "cdk deploy"

```bash
Anvil ec2: 18.118.32.254
aws ssm start-session --target i-0e3242cc893c86a25

Node ec2: 18.224.44.15
aws ssm start-session --target i-0da73558ad639280b

Env variables for contract deployment:
export ETH_RPC_URL="http://18.118.32.254:8545"
export NODE_RPC_IP="18.224.44.15"
```

Note: After cdk deploy finishes running, the configure_node.sh userdata script make take some more time to complete. Please wait for "complete" to appear in the /tmp directory.

## Anvil EC2 Instructions

The anvil container should start automatically. If you need to monitor logs or restart the chain, please see the following.

```bash
# connect to anvil ec2 instance
aws ssm start-session --target i-0e3242cc893c86a25 # see stack outputs

# monitor anvil 
docker logs -f <containerid>

# Restart chain
docker pull arpachainio/anvil-test:latest
docker run -d --name anvil-chain -p 8545:8545 arpachainio/anvil-test:latest
```

## Node EC2 Instructions

The node container requires a few steps to start. First, you will need to deploy the randcast smartcontracts to our anvil chain using the contract-init container.

```bash

# Connect to node ec2
aws ssm start-session --target i-0da73558ad639280b # see stack outputs

# Prep environment
cd /tmp/BLS-TSS-Network/docker/internet-test/arpa-node
export ETH_RPC_URL="http://<ETH_RPC_IP>:8545" # see stack outputs
export NODE_RPC_IP="<NODE_RPC_IP>" # see stack outputs
./prep_config.sh # prep node config files with the above env values.
# config_1.yml:
# Prior values:
#  . provider_endpoint: "http://0.0.0.0:8545" 
#  . node_advertised_committer_rpc_endpoint: "0.0.0.0:50061" 
# Updated values:
#  . provider_endpoint: "http://3.142.54.57:8545"
#  . node_advertised_committer_rpc_endpoint: "18.188.109.131:50061"   etc....

# NOTE: This is a sample script only. For production, you would need to edit these values in addition to your wallet private key and other parameters. Please see the root level README for more details. 

# run container-init to deploy contracts
docker run -d --name contract-init -v /tmp/BLS-TSS-Network/contracts/.env.example:/usr/src/app/external/.env -e ETH_RPC_URL=$ETH_RPC_URL arpachainio/contracts-test:latest
# monitor logs to see when contract deployment is complete
docker logs -f contract-init
```

Once the contract-init container has finished deploying the contracts, you can start the node containers.


```bash

# run arpa node containers
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v /tmp/BLS-TSS-Network/docker/internet-test/arpa-node/config_1.yml:/usr/src/app/external/config.yml arpachainio/node:latest
docker run -d --name node2 -p 50062:50061 -p 50092:50091 -v /tmp/BLS-TSS-Network/docker/internet-test/arpa-node/config_2.yml:/usr/src/app/external/config.yml arpachainio/node:latest
docker run -d --name node3 -p 50063:50061 -p 50093:50091 -v /tmp/BLS-TSS-Network/docker/internet-test/arpa-node/config_3.yml:/usr/src/app/external/config.yml arpachainio/node:latest

# check if nodes grouped succesfully
docker exec -it node1 /bin/bash       
watch 'cat /var/log/randcast_node_client.log | grep "available"'

# deploy user contract / check if randomness worked.
docker exec -it contract-init /bin/bash
forge script /usr/src/app/script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url $ETH_RPC_URL --broadcast
cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 "getLastRandomness()(uint256)" # should not show 0
cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e "lastRandomnessResult()(uint256)" # should match above
```

## Teardown

```bash
cdk destroy
```

## Useful Commands

The node-test ec2 comes with a dockerized version of foundry. You can run foundry commands using it like this:
[docs](https://book.getfoundry.sh/tutorials/foundry-docker)

```bash
# run cast using foundry docker image (included in node-test ec2)
docker run foundry "cast block --rpc-url $RPC_URL latest"

# Cleanup Docker images
docker kill $(docker ps -q)
docker rm $(docker ps -a -q)
docker kill $(docker ps -q -f ancestor=arpachainio/node:latest) # kill node containers

```
