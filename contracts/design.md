# Controller Flow / Smart Contract Design

This note explores all the components of the controller contract and how they interact with each other.
(Note: need to draw a new design diagram)

We will consider if we can have an improved design for solidity contract.

Can we get rid of anything?

## Function List and Definitions

```rust
fn node_register(id_address: String, id_public_key:Vec<u8>) -> ControllerResult<()> 
// This function is called by a node to register itself with the controller.
// Create instance of node struct:  add node to nodes map / rewards map
// call node_join

fn node_join(id_address: String) -> ControllerResult<bool> 
// call find_or_create_target_group to get group_id and check if we need to reblance

fn add_group() -> usize 
// increment group index, populate group struct, insert group to groups map.

fn add_to_group(node_id_address: String, group_index: usize, emit_event_instantly: bool) -> ControllerResult<()> 
// create member struct from node_id_address, add to members map
// set minimum to minimum_threshold

fn minimum_threshold(n: usize) -> usize {(((n as f64) / 2.0) + 1.0) as usize}
// The minimum allowed threshold is 51%

 fn emit_group_event(group_index: usize) -> ControllerResult<()> 
// Increment group epoch: Number of times a group event was emitted for a particular group
// Increment epoch: Sum of all existing group epochs (version number tor synchroniztion between nodes, smart contract, and diff chains)
// Deploy new coordinator -> Initialize coordinator with members -> insert coordinator into the coordinator map
// Emit Event to kick off dkt task for nodes. 

fn rebalance_group(mut group_a_index: usize, mut group_b_index: usize) -> ControllerResult<bool> 
// Rebalance groups?? How does it work?

fn emit_dkg_task(&self) -> ControllerResult<DKGTask> {
// ?? How does this work? Why is it commented out

fn valid_group_indices(&self) -> Vec<usize> {
// ? 


```

## Rust Functions Flow

```bash
node_register:
  node_join:
    find_or_create_target_group
      valid_group_indices
      add_group
    add_to_group
      minimum_threshold  
      emit_group_event
        emit_dkg_task
    reblance_group
      choose_randomly_from_indices
      remove_from_group
      add_to_group
      emit_group_event
```

## Questions

### Specific Questions

Need some Rust explanation on these variables:

- node_join (line 228): is this just getting all keys (group_index) from the groups map?
- find_or_create_target_group (line 318): is this group_index's from the groups map sorted on g.size?

### Missing functions?

What do these functions do:

- Rebalance Group?
- Emit DKG Task?

``` Rust
let dkg_task = DKGTask {
    group_index: group.index,
    epoch: group.epoch,
    size: group.size,
    threshold: group.threshold,
    members,
    assignment_block_height: self.block_height,
    coordinator_address: self.deployed_address.clone(),
};
self.dkg_task = Some(dkg_task);
// self.emit_dkg_task(dkg_task);
Ok(())
```

Where are all the other functions called?

- emit_dkg_task
- commit_dkg, post_process_dkg etc...)



### Design: Coordinator interface

emitGroupEvent: Epoch isn't in coordinator constructor.. is it needed?? (coordinator.sol line 114)


### Trimming structs

Can group and node structs be trimmed down?

```rust
let group = Group {
    index: group_index,
    epoch: 0,
    capacity: GROUP_MAX_CAPACITY,
    size: 0,
    threshold: DEFAULT_MINIMUM_THRESHOLD,
    is_strictly_majority_consensus_reached: false,
    public_key: vec![],
    fail_randomness_task_count: 0,
    members: BTreeMap::new(),
    committers: vec![],
    commit_cache: BTreeMap::new(),
};
```

What is needed in node struct?

```rust
let node = Node {
    id_address: id_address.clone(),
    id_public_key,
    state: true,
    pending_until_block: 0,
    staking: NODE_STAKING_AMOUNT,
};
```

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

## Completed

Implement

- [x] minimum threshold calculations
- [x] Require statements for node register and emit_group_event
- [x] Create public test functions for private functions

Test "require" statements:

- [x] test fail: emit group event for non existent group index
- [x] test fail: register a node that has already been registered 

add to group

- [x] minimum = minimum threshold(group.size)

emit group event

- [x] Require !groups.contains_key(&group_index)

