---
type: tech
keywords: 
tags: 
---
# randcast gas optimization 

Thursday, April 20, 2023

---

## Adapter.sol

line 358 (reward randomness)

line 273 callback struct (store information about user request in callback struct)

- We can avoid storing so much data
- Compute and store hash of all this data
  
Node passes all the data we need.

## Focus

Reduce gas costs for

- requestRandomness()
- fulfillRandomness()

## Pre-req: Find a way to calculate real consumption amount

- Determine if there is a relaationship between forge gas estimate and actual gas (fixed ratio)

1. find a way to calculate real consumption amount
   1. Deploy contracts to testnet
   2. Mock some nodes somehow
   3. Call the functions.

## Another example

ConsumerRequestBalance.t.sol

```solidity
    // TODO this is not confirmed
    uint32 gasExceptCallback = 575000;
```

all gas in fulfillrandomness except for user defined callback functions.

`bool success = fulfillCallback(requestId, randomness, callback);`

We can't control this part. We can't control the gas consumption of the user defined callback function. But we can calculate the gas consumption of the rest of the function.

callback funcitons in `BasicRandcastConsumerBase.sol`

```js
// these 3 functions are for users to overwrite
    function fulfillRandomness(bytes32 requestId, uint256 randomness) internal virtual {}

    function fulfillRandomWords(bytes32 requestId, uint256[] memory randomWords) internal virtual {}

    function fulfillShuffledArray(bytes32 requestId, uint256[] memory shuffledArray) internal virtual {}

// these are for the adapter contract to call
    function rawFulfillRandomness(bytes32 requestId, uint256 randomness) external {
        require(isEstimatingCallbackGasLimit || msg.sender == adapter, "Only adapter can fulfill");
        fulfillRandomness(requestId, randomness);
    }

    function rawFulfillRandomWords(bytes32 requestId, uint256[] memory randomWords) external {
        require(isEstimatingCallbackGasLimit || msg.sender == adapter, "Only adapter can fulfill");
        fulfillRandomWords(requestId, randomWords);
    }

    function rawFulfillShuffledArray(bytes32 requestId, uint256[] memory shuffledArray) external {
        require(isEstimatingCallbackGasLimit || msg.sender == adapter, "Only adapter can fulfill");
        fulfillShuffledArray(requestId, shuffledArray);
```

## notes on request randomness

User needs to call requestRandomness in their contract and then define usage since both need to happen in same transaction.

There are 2tx in a single randomness requesting proccess:

- request randomness triggered by user (users signs tx and broadcasts)
- fulfill randomness triggered by commiter (commiter signs tx and broadcasts)

User needs to calculate the gas cost of fulfill randomness, and notify the nodes by passing the gas cost to the requestRandomness function.

feature imrpovement: we try to calculate gas callback cost for user and pass it to the requestRandomness function. (GeneralRandcastConsumerBase.sol -> abstract contract for users to extend)

---

gnosis

用途 usage

---
---

## Problem

What is the exact gas cost of fulfillRandomness() - fulfillCallback()

This is good because....

1. A good soliditiy developer is nice at gas.
2. We need to save gas for the user!