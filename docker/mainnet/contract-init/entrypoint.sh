#!/bin/bash
/root/.foundry/bin/forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://0.0.0.0:8545 --broadcast
/root/.foundry/bin/forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://0.0.0.0:8545 --broadcast -g 150
tail -f /dev/null

## use cmd here so that anvil ip can be passed in
## start uploading images to docker hub