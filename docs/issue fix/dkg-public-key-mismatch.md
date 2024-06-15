# How To Fix DKG Public Key Mismatch Issue

## Symptom

The node-client program exits with an error log: "Node is registered with different dkg public key". This indicates that your Node account has been registered and the local database file used for the current startup command does not match the information registered on-chain.

## General Guidance

Since there is a mismatch, all we are trying to do here is to remove the mismatch or make they sync up.

- If the original DB file still exists (which means you maybe just used wrong command), we can try to sync it up
- If it does not exist anymore (which means the file is damaged or deleted), we can try to remove it and reset DKG key on-chain.

Below steps require manual operation with DB file and on-chain contract. For the contract part, you are going to need

- [NodeRegistry Contract](https://github.com/ARPA-Network/BLS-TSS-Network/blob/0732850fe39f869a7dea899e445dfe6332462ab7/contracts/src/interfaces/INodeRegistry.sol)
- The address of the contract is listed in our [Official Document](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters)

## Detailed Steps

0. Stop your existing Docker instance and remove it.
1. Confirm your node status as non-working:
   - Go to NodeRegistry and call getNode and check the second last value of the `Node` struct
   - If `state` is true: call `nodeQuit` by `Node` account to prepare for changing DKG key before continue. Please refer to **Exit Node from ARPA Network** section in our [onboarding doc](/docs/eigenlayer-onboarding.md) for more details.
2. Database file handling, as mentioned:

   - If the original DB file does not exist and the file you currently have causes the mismatch, delete your current database file.
   - If the original DB file still exists, you can try to re-run as long as start command points to it correctly.

3. Pull the most recent version of the ARPA Node Client Docker image by running the following command:

   `docker pull ghcr.io/arpa-network/node-client:latest`

4. Start the Docker container by running the command in [onboarding doc](/docs/eigenlayer-onboarding.md) and wait about 1 minute for the node-client to fully start.

`Note`: If your database file is named differently than "data.sqlite", you need to rename it since by default node client looks for the "data.sqlite" file.

#### For people with original DB file, skip step 5-7

5. You should expect the program to exit with error log "Node is registered with different dkg public key", search the log for the keyword "public_key"(or "DKGKeyGenerated") and copy the public key value.
6. Call the `changeDkgPublicKey` contract method manually by `Node` account with your generated public key.
7. Re-run your node-client

#### Rejoin ARPA network

8. Generate the EIP1271 Operator signature for AVS registration with your `Asset` Account (for details, check our [onboarding doc](/docs/eigenlayer-onboarding.md#generate-the-eip1271-operator-signature-for-avs-registration-with-your-asset-account))
9. Call `nodeActivate` method by `Node` account to activate your node again.
10. You should now expect it to group correctly
