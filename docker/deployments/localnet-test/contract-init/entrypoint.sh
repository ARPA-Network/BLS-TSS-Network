#!/bin/bash
/root/.foundry/bin/forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://anvil-chain:8545 --broadcast
/root/.foundry/bin/forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://anvil-chain:8545 --broadcast -g 150
tail -f /dev/null