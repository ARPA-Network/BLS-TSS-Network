# Extending DKG Scenario Tests

LGTM. Good job! We made a great progress to have 95% code coverage.

Next, some cases still need to be covered. The first is other scenarios triggering dkg process. For now all DKG scenarios are triggered by node register, while they can also be triggered by node exit and rebalance happening in node register/exit. Also, to cover rebalance cases, we have to do some setup work to set the context of network.

I know it is a hard work, but I think it's more proper to have them covered in forge testing environment instead of integration test involving controller contract, user contract and nodes. Please tell me if there is anything I can help.

I think we can merge this pr first. Then I will incorporate staking contract into the current repo. It may take some changes into the current codebase but we can mock the staking contract while we test the controller to keep the test cases easy to go. Let's keep moving simultaneously.

---

## Rebalance Tests

`Regroup Single (5) -> nodeQuit`
group_0: 5 members
1 member of group_0 wants to exit the network
Then, controller will let 4 members left in group_0 do dkg as 4 > 3 which is the threshold
i.e. the group still meet the grouping condition
after that, in happy path group_0 will be functional with 4 members.

`Rebalance Two Groups (5,3) -> nodeQuit -> 3,4)`
group_0: 5 members
group_1: 3 members
1 member of group_1 wants to exist the network.
Then, controller will let group_1 which has 2 remaining members rebalance with group_0.
Result: group_0 (3 members), group_1 (4 members) are both functional.

In above cases we need to pay attention to pathes containing rebalance, and please assert group.epoch in each case. Since you have tested different cases in a single dkg process, I think we can cover only happy path in above scenarios, or the better, all the pathes we have for now.

## Grouping Tests

Second, we need to test the strategy of grouping. For now we have a new-started network and 5 fresh nodes to register, but what if we already have some nodes and groups in the network and some node wants to register and some node wants to leave? We need to assert controller's behavior of grouping decision, as to let what nodes to do the next round of dkg as group x, epoch n+1.

(in the following cases we set groupMaxCapacity as 6 instead of 10 to make it simpler to setup, you can change it in controller setting)

`(6,6) -> nodeRegister -> (3,6,4)`
group_0: 6 members
group_1: 6 members
A new node calls nodeRegister.
Controller should create a new group (group_2), add the new node into group_2, and then rebalance between group_0 and group_2.
Final network status should be (3,6,4), all functional, group_0 and group_1 should be grouped before next round of commitDkg.

`(3,3) -> nodeRegister -> (3,3,1)`
group_0: 3 members
group_1: 3 members
A new node calls nodeRegister
Controller should create a new group_2, add the new node into group_2, then try to rebalance. 
Final network status should be (3,3,1) with group_2 not yet functional.

`ideal number of groups` (How do i get these network into state (3,3,3,3,3)
(5 groups) group 0-4 have 3 members each (3,3,3,3,3)
A new node calls nodeRegister
Controller should add new node into group_0 resulting in network state (4,3,3,3,3).
group_0 should be functional.

`ideal number of groups at max capacity`
(5 groups) group 0-4 have 6 members each (6,6,6,6,6)
A new node calls node register.
Controller should create a new group (group_5), add the new node to group_5, and then rebalance between group_0 and group_5.
Resulting network state should be (3,6,6,6,6,4) group_0 should be functional.

`(6,3) -> group_1 nodeQuit -> (4,4)`
This may looks similar to the first case, but please note that the paths to initial status (3,6) and (6,6,6,6,6) are different and they are both reachable in real cases.

group_0: 6 members
group_1: 3 members
member in group_1 called nodeQuit.
Then, the controller should rebalance between group_0 and group_2,
Resulting network state should be (4,4)
Borth group_0 and group_1 should be functional.

`(6,3) -> group_0 nodeQuit -> (5,3)`
group_0: 6 members
group_1: 3 members
member in group_0 calls nodeQuit.
Then, the controller should emitDkgEvent for group_0 with the 5 remaining remmbers.
Resulting network state should be (5,3)
Group_0 should be functional.

---

## Generating DKG Public Keys

```bash
cd /Users/zen/dev/pr/BLS-TSS-Network/crates/threshold-bls/src/curve
bn
cargo test --test serialize_field

# prevent supression of output
cargo test -- --nocapture

# Run only a specific unit test
cargo test serialize_group -- --nocapture

# Note: investigate /Users/zen/dev/pr/BLS-TSS-Network/crates/threshold-bls/src/test_bls.rs
```

Edit bn254.rs

```rust

// serialize_group_test::<G1>(32); // line 436 commented out

// line 441
use ethers_core::{types::U256, utils::hex}; // ! new

    fn serialize_group_test<E: Element>(size: usize) {
        let empty = bincode::deserialize::<E>(&vec![]);
        assert!(empty.is_err());

        let rng = &mut rand::thread_rng();
        let sig = E::rand(rng);
        let ser = bincode::serialize(&sig).unwrap();
        // println!("{:?}", ser); // print serialized value
        println!("bytes DKGPubkey1 = hex{:?}", hex::encode(ser.clone()));

        assert_eq!(ser.len(), size);

        let de: E = bincode::deserialize(&ser).unwrap();
        assert_eq!(de, sig);
    }
```

## partialPublic key vs DKGPublicKey, publicKey

DKGPublicKey: interaction between DKG proccess. Each node generates one for itself a single time. Uses it to register to network. Key is fixed for the life of the node. 

publicKey / partialPublicKey are generated by the DKG proccess. 
Both keys are commited during commitDkg function call. 
The nodes receive a grouping event, and then they participate in dkg and generate the two keys. 

3 nodes register, controller emits event.
all nodes receive the event: which container nodeIdAddress, DKGPublicKey.
all nodes generate partialPublicKey and publicKey (it knows which nodes to talk to from the nodeIdAddresses in the emitted event), this is in preparation for commitDkg.
Nodes then call commitDkg

## For assertions

make sure that the group.epoch is correct.

every step after node register or exit, check that the group.epoch is correct.
