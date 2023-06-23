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
docker pull arpachainio/anvil-test:latest
docker run -d --name anvil-chain -p 8545:8545 arpachainio/anvil-test:latest

# create complete file
touch /tmp/complete

