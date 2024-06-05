# How To Fix DKG Public Key Mismatch Issue

## General Guidance

Since there is a mismatch, all we are trying to do here is to remove the mismatch or make they sync up.
- If the original DB file still exists, we can try to sync it up
- If not, we can try to remove it and reset DKG key on-chain.

Below steps require manual operation with DB file and on-chain contract. For the contract part, you are going to need 
- [NodeRegistry Contract](https://github.com/ARPA-Network/BLS-TSS-Network/blob/0732850fe39f869a7dea899e445dfe6332462ab7/contracts/src/interfaces/INodeRegistry.sol)
- The address of the contract is listed in our [Official Document](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters)

## Detailed Steps

0. Stop your existing Docker instance and remove it.
1. Confirm your node status. 
    - Go to NodeRegistry and call getNode and check the second last value of the `Node` struct
    - If value is true: call nodeQuit to clear the status before continue. (If it fails, contact us via telegram group)
2. Database file handling, as mentioned:
    - If the original DB file does not exist and the file you currently have causes the mismatch, delete your current database file.
    - If the original DB file still exists, you can try to re-run as long as start command points to it correctly.

3. Pull the most recent version of the ARPA Node Client Docker image by running the following command:

    ```docker pull ghcr.io/arpa-network/node-client:latest```

4. Start the Docker container by running the following command: 
    ```bash
        docker run -d \
        --name <name of your node> \
        -v <path of config file>:/app/config.yml \
        -v <path of DB folder>:/app/db \
        -v <path of log folder>:/app/log \
        --network=host \
        ghcr.io/arpa-network/node-client:latest
    ```
``Note``: If your database file is named differently than "data.sqlite", you need to rename it since by default node client looks for the "data.sqlite" file.

#### For people without original DB file, please continue.

5. After about 1 minute, you should expect to see "Node is registered with different dkg public key" error, search the log for the keyword "public_key"(or "DKGKeyGenerated") and copy the public key value.
6. Call the changeDkgPublicKey contract method manually with your generated public key. 
7. Stop and re-run your node-client
8. Generate a signature and call nodeActivate method to activate your node again
8. You should now expect it to group correctly