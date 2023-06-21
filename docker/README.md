# Randcast DevOps Scripting (Docker / CDK IAC)

## Overview

This folder contains the dockerfiles, scripts, and config files for facilitating streamlined randcast deployments (both for testing and production use on mainnet).

## Folder Structure

Scripts are split into the following sections:

[localnet-test](./localnet-test/README.md): deploy testnet locally using docker network

[internet-test](./internet-test/README.md): deploy testnet online using AWS EC2 + docker

[mainnet](./mainnet/README.md): for node operators to deploy their nodes on mainnet (AWS EC2 + docker)

For the internet-test and mainnet folders, We have also included CDK scripts for deploying AWS EC2 instances along with the neccesary networking and IAM configurations to host your node containers.

## Images offerred

ARPA/anvil-dev
ARPA/contract-deployment-dev
ARPA/node-dev
ARPA/node-prod (edited)

