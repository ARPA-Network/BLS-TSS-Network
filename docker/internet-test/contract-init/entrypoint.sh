#!/bin/bash

# Env variable passed in on docker run, see readme.
ETH_RPC_UR=${ETH_RPC_URL}

# Move .env into contracts folder for solidity scripts to use.
cp /usr/src/app/external/.env /usr/src/app/.env

/root/.foundry/bin/forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url $ETH_RPC_URL --broadcast
/root/.foundry/bin/forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url $ETH_RPC_URL --broadcast -g 150
tail -f /dev/null