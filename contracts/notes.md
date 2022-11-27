# Notes

## Remaining Todo (from ruoshan code review)

- [ ] Refactor CommitDkg params to a struct to save local variable usage.
- [ ] Test selfDestruct once Forge feature is available.
- [ ] Check the format of publicKey / partialPublicKey (need to find sol library)
- [ ] Double check if "delete" keyword is working properly for clearing commiters and commitcache.
- [ ] Merge rewards map into node struct (hold for now)

## Check the format of publickey / partial public key

Format is subjected to the ECC specification that BLS12-381 used. Here we need to find a BLS12-381 library in solidity and look through their function to verify the format as far as possible. Need to find a suitable library.

[BLS12-381 For The Rest Of Us](https://hackmd.io/@benjaminion/bls12-381)
[b12-sol](https://github.com/prestwich/b12-sol)
[b12_381 rust](https://github.com/zkcrypto/bls12_381)

Todo:

- [ ] Research b12_381
- [ ] Figure out possible ways to verify format of public key / partial public key

## "Delete" Research (emitGroupEvent)

- [ ] Test how [delete](https://ethereum.stackexchange.com/questions/35909/using-delete-keyword-on-storage-variables/35993#35993) works! (179, 180)
- [ ] It seems like delete might not be doing anything. need to track down exactly when emitGroupEvent is triggered, I think it's triggering too often?

---

## Todo (New Developments)

- [ ] Implement Reblance Group
- [ ] Implement index_of_min_sixe
- [ ] Emit DKG Task in emitGroupEvent()
- [ ] commitDKG : Last parts
- [ ] postProccessDkg : Group Relay Task

## Questions for meeting:

- How much about BLS12-381 do we need to know?
- Could you help explain controller.rebalanceGroup()?
- some hints on emit dkg task event?