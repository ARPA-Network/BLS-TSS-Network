# Controller Deep Dive

## Function Flow

``` rust
node_register(ip, pubkey) -> ControllerResult<()>  
  node_join(ip) -> ControllerResult<bool>
    find_or_create_target_group() -> (usize,bool)
      add_group() -> usize
      valid_group_indices() -> Vec<usize> // in adapter
    add_to_group(id, group_index, emit_inst:true) --> ControllerResult<()>:
      minimum_threshold(group.size) -> usize
      emit_group_event(group_index) -> ControllerResult<()>
    rebalance_group(group_a_index, group_b_index) -> ControllerResult<(bool)>
      choose_randomly_from_indices(seed, indices, expt_size) -> Vec<usize>
      remove_from_group(id, group_index, emit_inst) -> ControllerResult<(bool)>
      add_to_group(id, group_index, emit_inst) -> ControllerResult<(bool)>
      emit_group_event(group_a_index) -> ControllerResult<()>
      emit_group_event(group_b_index) -> ControllerResult<()>

emit_group_event()
  self.dkg_task = Some(dkg_task);
  //  self.emit_dkg_task(dkg_task); ? 

// Nodes work with coordinator to complete DKG task.

// At end of DKG phases....
commit_dkg(id, g_index, g_epoch, pubkey, partial_pubkey, disqualified_nodes) // nodes commit on dkg SUCCESS
  get_strictly_majority_identical_commitment_result(group_index)  // Skip detailed implementation for now, ask mocked node to send some identical values. 
  choose_randomly_from_indices(seed, indices, expt_size) -> Vec<usize>
  slash_node(id, staking_penalty...)

// After node calls commit_dkg, it will be monitor group state to see if DKG was succesful (via view functions on adapter 123 / 125)
    fn get_group(&self, index: usize) -> Option<&Group>;  // ! 
    fn get_group_state(&self, index: usize) -> bool; // !
    // If group state is failed at the end of the dks process (shortage of identical results), then node should call post_proccess_dkg

post_process_dkg(id, group_index, group_epoch) -> ControllerResult<()> //nodes call this on FAIL (does not mean node was dishonest, could have just failed DKG due to other nodes)
  get_strictly_majority_identical_commitment_result(group_index)
    slash_node(id, staking_penalty...) // fail
  // On success -> emit group_relay_task ?  (959) -> skip for now.
  
// freeze_node() // internal function for controller. Happens after misbehavior, related to slashnode. 
// node_activate() // Allows node to rejoin the network if it registered before and called node_quit() / of if it was slashed 
// node_quit()  // like node register. Allows node to quit from the network.
// report_unresponsive_group() 

// NOTE
// Node_register, commit_dkg, post_proccess_dks are independent. Each is called in a separate transaction. 
// Integtration Tasks: Can we test the code with some mock code for node behavior?
// Light-weight Mocked Node Utility: Call contract via ether.rs and run against anvil. 
    // Skip crytographic stuff just submit the right / wrong result (goal is to test the management proccess)
    // Mock node will call node_register, do dkg with coordinator, commit_dkg, post_proccess_dks
    // Contracts may be upgraded, would be good ot have lightweight node mock framework

```

3 cases:

- commit_dkg is called
  - Majority
  - Minority (less results than threshold) -> all nodes will be waiting.
- commit dkg not called, but post_process_dkg is called
- neither is called -> 

## Thoughts

Controller coordinates nodes registered to network
Nodes run dkg protocol to finish grouping.

In mock environment, nodes keep querying controller for latest grouping task, so cnroller only needs to store the task. Real controller contract needs to emit the task event for nodes to query. (So this hasn't been implemented yet?)

After: emit_group_event() -> emit_dkg_task(dkg_task), node_register returns. 
Nodes perform DKS and then call commit_dkg(). 
As soon as the last phase ends (success or failure), nodes call post_process_dkg() to...???

Things don't happen in a single transaction.


## Questions

``` rust
// 323 (find_or_create_target_group)
        let (index_of_min_size, min_size) = self
            .groups
            .values()
            .map(|g| (g.index, g.size))
            .min_by(|x, y| x.1.cmp(&y.1))
            .unwrap();

// 228 (node_join)
        let group_indices = self
            .groups
            .keys()
            .copied()
            .filter(|i| *i != group_index)
            .collect::<Vec<_>>();

// 271 (emit_group_event)
        let mut members = group
            .members
            .values()
            .map(|m| {
                let public_key = self.nodes.get(&m.id_address).unwrap().id_public_key.clone();
                (m.id_address.clone(), m.index, public_key)
            })
            .collect::<Vec<_>>();

```