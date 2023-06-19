#!/bin/bash

# use the value of the `RPC_ENDPOINT` environment variable if it's provided; otherwise, it will use the default value `http://0.0.0.0:8545`.
RPC_ENDPOINT=${RPC_ENDPOINT:-http://0.0.0.0:8545}

/root/.foundry/bin/forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url $RPC_ENDPOINT --broadcast
/root/.foundry/bin/forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url $RPC_ENDPOINT --broadcast -g 150
tail -f /dev/null