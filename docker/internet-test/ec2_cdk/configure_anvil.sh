#!/bin/sh
# Set hostname
HOSTNAME=anvil-test
echo "Setting the hostname to $HOSTNAME"
hostnamectl set-hostname $HOSTNAME
echo "127.0.0.1 $HOSTNAME" >> /etc/hosts

# install docker
sudo yum update -y
sudo yum -y install docker git
sudo service docker start
sudo usermod -a -G docker ec2-user
sudo chmod 666 /var/run/docker.sock
sudo systemctl enable docker.service # enable docker on boot
sudo systemctl start docker.service # start docker
docker ps


# pull and run anvil
docker pull wrinkledeth/anvil-chain:latest
docker run -d --name anvil-chain -p 8545:8545 wrinkledeth/anvil-chain:latest

# # install foundry
# curl -L https://foundry.paradigm.xyz | bash && \
#     /home/ssm-user/.foundry/bin/foundryup
# source /home/ssm-user/.bashrc

# # clone repo
# git clone -b dockerAutomation https://github.com/wrinkledeth/BLS-TSS-Network.git
# cd /tmp/BLS-TSS-Network/contracts # this goes to temp directory
# forge test

# # pull container-init and arpa-node images
# docker pull wrinkledeth/anvil-chain:latest
# docker pull wrinkledeth/contract-init:latest
# docker pull wrinkledeth/arpa-node:latest

# create complete file
touch /tmp/complete

# debugging userdata script
# sudo cloud-init status
# sudo cat /var/log/cloud-init-output.log
