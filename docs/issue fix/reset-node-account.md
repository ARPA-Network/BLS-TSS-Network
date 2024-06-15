# Reset Node Account

1. Is the current `Node` account in a working state? Y->2 N->5
2. Is the current `Node` account in grouping DKG process, which exceeded 40 block heights and has not yet ended? Y->3 N->4
3. Use the old `Node` account to call Controller.postProcessDKG(uint256 groupIndex, uint256 groupEpoch).
4. Use the old `Node` account to call NodeRegistry.nodeQuit().
5. Is the `Node` account in the previously used Node client configuration file already new? Y->8 N->6
6. Stop the node-client, delete the database, and modify the `Node` account in the configuration file to the new one that needs to be reset.
7. Rerun node-client, search the log for the keyword "public_key"(or "DKGKeyGenerated") and copy the latest DKG public key value.
8. (Keep node-client running) Use `Asset` account to call NodeRegistry.nodeLogOff().
9. Use the new `Node` account to call NodeRegistry.nodeRegister(...), refer to [Node Registration](/docs/eigenlayer-onboarding.md#register-to-arpa-network-by-your-node-account) for details.

Note:

- Check the status of the `Node` account by calling `getNode(address nodeAddress)(Node node)` and check the second last value of the `Node` struct in the `NodeRegistry` contract. True means the `Node` account is working.
- Check the membership status of the `Node` account by calling `getBelongingGroup(address nodeAddress)(uint256 groupIndex, uint256 memberIndex)` in the `Controller` contract. If the result is not -1, the `Node` account is a member of a group.
- Check if the DKG process has ended by calling `getCoordinator(uint256 groupIndex)` in the `Controller` contract. If the result is not 0, the DKG process is still ongoing.
- Based on the `groupIndex`, choose either way below to find the latest `groupEpoch`:

  - Call `getGroup(uint256 groupIndex)(Group group)` in the `Controller` contract and find the `epoch` field(the second value in the `Group` struct).
  - Find the latest `groupIndex` and `groupEpoch` in the node-client log, by searching for the keyword `DKGGroupingStarted` and the `group_log` field. If your node missed the DKG process, choose the way above.

- Check the start block height of the DKG process by searching the `DKGTask` event on-chain with the `groupIndex` and `groupEpoch` in the `Controller` contract. Generally, this is automatically done by the node-client, but if the current block height exceeds the start block height by 40 blocks, you can call `Controller.postProcessDKG(uint256 groupIndex, uint256 groupEpoch)` manually to finalize the DKG process. There is a reward for doing this, refer to [Reward](/docs/eigenlayer-onboarding.md#reward) for details.
