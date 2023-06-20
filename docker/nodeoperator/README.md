
# Create EC2 Instance for Randcast Node Operator

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

## Build Docker Image, tag, and push to dockerhub

```bash
# build image from source
docker build -t arpa-node-prod ./docker/nodeoperator/arpa-node

# tag image
docker tag arpa-node-prod:latest wrinkledeth/arpa-node-prod:latest

# pull images from repo
docker pull wrinkledeth/arpa-node-prod:latest
```

## Sample Workflow

```bash
cdk deploy

# connect to ec2 instance
aws ssm start-session --target i-091e4058778037871

# clone repo
git clone -b dockerAutomation https://github.com/wrinkledeth/BLS-TSS-Network.git
cd /tmp/BLS-TSS-Network/


# run arpa-nodes (Make sure config files are configured properly)
docker run -d --name node1 -p 50061:50061 -p 50091:50091 -v /tmp/BLS-TSS-Network/docker/nodeoperator/arpa-node/config_1.yml:/usr/src/app/external/config.yml wrinkledeth/arpa-node-prod:latest
docker run -d --name node2 -p 50062:50061 -p 50092:50091 -v /tmp/BLS-TSS-Network/docker/nodeoperator/arpa-node/config_2.yml:/usr/src/app/external/config.yml wrinkledeth/arpa-node-prod:latest
docker run -d --name node3 -p 50063:50061 -p 50093:50091 -v /tmp/BLS-TSS-Network/docker/nodeoperator/arpa-node/config_3.yml:/usr/src/app/external/config.yml wrinkledeth/arpa-node-prod:latest

# check if nodes grouped succesfully
docker exec -it node1 /bin/bash       
watch 'cat /usr/src/app/log/1/node.log | grep "available"'
cat /var/log/randcast_node_client.log

```

## Useful Commands

The node-test ec2 comes with a dockerized version of foundry. You can run foundry commands using it like this:
[docs](https://book.getfoundry.sh/tutorials/foundry-docker)

```bash
# run cast using foundry docker image
docker run foundry "cast block --rpc-url $RPC_URL latest"

# Cleanup Docker images
docker kill $(docker ps -q)
docker rm $(docker ps -a -q)
```
