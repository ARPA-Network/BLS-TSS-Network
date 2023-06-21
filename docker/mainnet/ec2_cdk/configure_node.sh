#!/bin/sh
# Set hostname
HOSTNAME=node-ec2
echo "Setting the hostname to $HOSTNAME"
hostnamectl set-hostname $HOSTNAME
echo "127.0.0.1 $HOSTNAME" >> /etc/hosts

# install dependencies
sudo yum update -y
sudo yum -y groupinstall "Development Tools" && sudo yum -y install pkgconfig libssh-devel docker git

# install docker
sudo service docker start
sudo usermod -a -G docker ec2-user
sudo chmod 666 /var/run/docker.sock
sudo systemctl enable docker.service # enable docker on boot
sudo systemctl start docker.service # start docker
docker ps

## install dockerized foundry (https://book.getfoundry.sh/tutorials/foundry-docker)
docker pull ghcr.io/foundry-rs/foundry:latest
docker tag ghcr.io/foundry-rs/foundry:latest foundry:latest
#docker run foundry "cast block --rpc-url $RPC_URL latest"

# clone repo
git clone https://github.com/ARPA-Network/BLS-TSS-Network
cd /tmp/BLS-TSS-Network/contracts

# pull container-init and arpa-node images
docker pull wrinkledeth/anvil-chain:latest
docker pull wrinkledeth/arpa-node:latest
docker pull wrinkledeth/contract-init:latest
docker pull wrinkledeth/arpa-node-prod:latest

# create complete file
touch /tmp/complete