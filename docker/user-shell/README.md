# Randcast User-shell

This folder contains tooling to develop and build a container that runs a user-cli tool for interacting with randcast contracts.

# Using the user-shell container
```bash
# pull image from dockerhub
docker pull arpachainio/user-shell:latest

# Run User Shell
docker run -it -v ./docker/user-shell:/data --network=host arpachainio/user-shell:latest "user-shell -c /data/user_config.yml -H /data/user-shell.history"

```

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

# Useful stuff

```bash
# Kill all containers
docker kill $(docker ps -q)
docker rm $(docker ps -a -q)
# rm all docker images
docker rmi $(docker images -q)

# exec into container for debugging
docker run -it -v /home/ubuntu/BLS-TSS-Network/docker/user-shell/user_config.yml:/usr/src/app/external/config.yml user-shell "bin/sh"
```