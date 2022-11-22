# Notes

## Overall Flow

Controller coordinates nodes registered to network
Nodes run dkg protocol to finish grouping.

In mock environment, nodes keep querying controller for latest grouping task, so controller only needs to store the task. Real controller contract needs to emit the task event for nodes to query. (So this hasn't been implemented yet?)

After: emit_group_event() -> emit_dkg_task(dkg_task), node_register returns. 
Nodes perform DKS and then call commit_dkg().
As soon as the last phase ends (success or failure), nodes call post_process_dkg() to...???

Things don't happen in a single transaction.

## TODO

node register

- [ ] Check to see if enough balance for staking
(What's going on here? Code seems to be gone)

node join

- [ ] Rebalance Group: Implement later?
(need explanation of how this works)

find or create target group

- [ ] Need to implement index_of_min_size  
(need explanation of how this works)

- [ ] is_strictly_majority_consensus, commit_cache, committers
(What is this stuff used for? do we need it?)

- [ ] group epoch isn't in coordinator constructor atm.

## Current Thoughts

getStrictlyMajorityIdenticalCommitmentResult gas optimization:
Thanks for the code! I think we can replace mapping struct during "word count" into two arrays. Furthermore to save gas for every commitment, this can be done before every new cache is recorded so that g.commitCache will be counted well. Maybe a little change to g.commitCache. Feel free to go ahead with rest part or adapter contract, and I will commit a pr later to improve this.

What is last_output and where is it generated? 


## Questions for ruoshan

- [ ] What is happening in reblance_group()? (line 372)
- [ ] What is actually happening in end of post_process_dkg()? (line 969)
- [ ] Is this okay: (g.members.length / 2) (line 207)
- [ ] Node Register -> Emit DKG Task. Need help on this (line 313)

---

## Ruoshan Changes

- CommitCache has multiple nodeIdAddress (address[]) instead of one (address),
- CommitCache: partialPublicKey removed.
- PartialKeyRegistered function doesnt look for specific partial key, just if one exists for the node.
- New Function: tryAddToExistedCommitCache:
- getStrictlyMajorityIdenticalCommitmentResult: returns commitCache instead of address[] (array of majority members with identical commit result)

## Questions for Ruoshan (Code Review 11/14/22)


## Remaining Todo (from ruoshan code review)

- [ ] Test how [delete](https://ethereum.stackexchange.com/questions/35909/using-delete-keyword-on-storage-variables/35993#35993) works! (179, 180)
- [ ] It seems like delete isnt doing anything. need to track down exactly when emitGroupEvent is triggered, I think it's triggering too often. 

## Check the format of publickey / partial public key

Format is subjected to the ECC specification that BLS12-381 used. Here we need to find a BLS12-381 library in solidity and look through their function to verify the format as far as possible. So until we find the lib I can't give more specific suggestions.

## Commit Language

