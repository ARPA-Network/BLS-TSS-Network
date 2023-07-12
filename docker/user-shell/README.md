# Randcast User-shell

This folder contains tooling to develop and build a container that runs a user-cli tool for interacting with randcast contracts.

# Building the container

```bash
# Deploy and connect to ubuntu dev env.
cd /home/ubuntu
git clone https://github.com/ARPA-Network/BLS-TSS-Network.git
cd /home/ubuntu/BLS-TSS-Network/ec2_cdk
cdk bootstratp
cdk synth
cdk deploy
ssh ubuntu@ip.com # place ssh public key in ec2_cdk/.env
# (wait for system message saying dev env is ready)

# Building container
cd /home/ubuntu/BLS-TSS-Network/
docker build -t user-shell ./docker/user-shell

# Tagging and uploading to dockerhub
docker tag user-shell arpachainio/user-shell:latest
docker push arpachainio/user-shell:latest
```

# Using the user-shell container
```bash
# pull image form dockerhub
docker pull arpachainio/user-shell:latest

# Run User Shell
docker run -it -v /home/ubuntu/BLS-TSS-Network/docker/user-shell/user_config.yml:/usr/src/app/external/config.yml arpachainio/user-shell:latest "/usr/src/app/user-shell -c /usr/src/app/external/config.yml"

# Run Cast Command
docker run --name cast -v /home/ubuntu/BLS-TSS-Network/docker/user-shell/user_config.yml:/usr/src/app/external/config.yml arpachainio/user-shell:latest "/root/.foundry/bin/cast <CASTCOMMAND>"


# Start Anvil (long running daemon)
docker run -d --name anvil -p 8545:8545 -v /home/ubuntu/BLS-TSS-Network/docker/user-shell/user_config.yml:/usr/src/app/external/config.yml arpachainio/user-shell:latest /root/.foundry/bin/anvil
```

# Useful stuff

```bash
# Kill all containers
docker kill $(docker ps -q)
docker rm $(docker ps -a -q)

# create permanent container for debugging
docker run -d --name perm -p 8545:8545 -v /home/ubuntu/BLS-TSS-Network/docker/user-shell/user_config.yml:/usr/src/app/external/config.yml user-shell "tail -f /dev/null"
docker exec -it perm /bin/bash
```