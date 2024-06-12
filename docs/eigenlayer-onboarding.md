# Node Setup Instruction for Eigenlayer Operator

## Node

The ARPA BLS-TSS Network consists of multiple groups of nodes. Within a group, each node is responsible for completing a BLS task (generating a BLS-TSS signature jointly with the other nodes of the group).

## Prerequisites

- **Minimum Hardware Requirements**
  Using AWS EC2 as an example, to ensure a steady performance, each node should be hosted on a virtual instance that meets the following specs:

  **t2.small (~$23/month)**

  - **1** vCPU
  - **2G** Memory
  - **30G** Storage

- [Docker](https://www.docker.com/get-started/)
- Externally accessible IP and ports

## EL Operator Setup

Follow the [official document](https://docs.eigenlayer.xyz/eigenlayer/operator-guides/operator-installation) to install and set up the operator accordingly.

After this, you should have:

- operator private key / address setup
- docker installed

## Staking and Whitelist

Due to the fact that Eigenlayer in the M2 phase does not support slashing, we collaborate with operators through a whitelist approach. Currently, there are no staking requirements set.

Nodes on the whitelist can join the ARPA Network and earn rewards by correctly performing the BLS tasks.

## Reward

Besides the cooperation agreement, there are three types of rewards a node can earn through smart contract if acted responsibly in a timely manner:

- Randomness submission reward
  - 1 ARPA each time a node successfully submits a randomness result.
- BLS-TSS task reward
  - 1 ARPA each time a node completes a BLS-TSS task.
- DKG post-process reward
  - 60 ARPA each time a node completes the DKG post-proccess task during node grouping.

## Basic Account Concepts

Before we start, let's talk about some basic concepts.

To serve both Eigenlayer and ARPA architecture, we have 2 types of account, `Asset` and `Node` account.

- Asset Account: **Eigenlayer Operator account**, which is used for registration check, and slashing purposes (not implemented yet).
- Node Account: **The account that interacts with ARPA**, which is used for identity in ARPA network, gas provision and rewards. You may use same account info as your asset account or create a new ECDSA account as you wish.

`Note`:

- Unlike other account pattern, the `Node` account is also very important, therefore you need to make sure the account secret is secured.
- Only the `Node` account identity needs to be provided in the configuration file of `node-client`.
- When the node is in a non-working state(exited or slashed), the `Asset` account can be used to reset the binding relationship of the `Node` account. Please refer to the [Log Off Node Account by Asset Account](#log-off-node-account-by-asset-account) section for more details.

## Balance

For gas fee of grouping operations and task submission, please ensure you have a sufficient ETH balance ready on all the L1/L2s `Node` account that you need to support, which currently are:

- Testnet
  - ETH Holesky (17000)
  - Redstone Garnet (17069)
- Mainnet
  - ETH Mainnet (1)
  - OP Mainnet (10)
  - Base Mainnet (8453)
  - Redstone Mainnet (690)

**_Note:_** The gas cost of fulfilling tasks(the signature verification and callback function) is directly paid by the committer node and then reimbursed by the requesting user. `Node` account can retrieve both the prepaid ETH and extra ARPA reward at any time by calling the `nodeWithdraw` method from the `NodeRegistry` contract on L1, or `ControllerOracle` contract on L2s.

## Setup Steps

### Build a config (yml file) in your running environment

Please copy the template below, and to change:

- **provider_endpoint (Necessary, both main chain and relayed chains)**
- **account (Necessary)**
- node_management_rpc_endpoint, node_management_rpc_token (Optional)
- listeners and time_limits (Optional and please modify carefully if needed)

**_Note:_**

- The contract addresses in the following example are currently the latest available.
- We recommend setup account by keystore or hdwallet. Please refer to [here](https://github.com/ARPA-Network/BLS-TSS-Network/tree/main/crates/arpa-node#node-config) for detailed instructions. The priority order is 1. hdwallet 2. keystore 3. private key
- example:
  ```yaml
  account:
    keystore:
      password: <KEYSTORE_PASSWORD>
      path: node.keystore
  ```
- Please **DO NOT use comments** in the config file.
- Please confirm that the `node_advertised_committer_rpc_endpoint` can be accessed by the external network.

**Testnet config.yml example**

[Config for Testnet](/docs/config.holesky.yaml)

**Mainnet config.yml example**

[Config for Mainnet](/docs/config.mainnet.yaml)

### Run below commands to start the `node-client`:

```bash
#!/bin/bash
cd <YOUR_ARPA_NETWORK_ROOT_DIRECTORY>

# Suppose you have a config.yml here
# Use `node-config-checker` to make the integrity check
# It will print out the address of the account provided in the configuration file,
# otherwise the error reason will be printed
docker run -v ./config.yml:/app/config.yml ghcr.io/arpa-network/node-config-checker:latest "node-config-checker -c /app/config.yml"

# Create the necessary directories if it's your first run
mkdir db
mkdir log

# Create the config.yml file and fill in config details in step #1

# Pull the latest Docker image
docker pull ghcr.io/arpa-network/node-client:latest

# Run the Docker container
## Parameters needs to be provided as below to your docker instance:
## 1. config file path on your host machine
## 2. DB folder path on your host machine
## 3. log folder path on your host machine
## 4. (Optional) Node account keystore file path on your host machine. If you provide keystore, you don't need to provide private key again in your config as the keystore will override private key.

docker run -d \
--name arpa-node \
-v <path of config file>:/app/config.yml \
-v <path of DB folder>:/app/db \
-v <path of node account keystore file>:/app/node.keystore \
-v <path of log folder>:/app/log \
--network=host \
ghcr.io/arpa-network/node-client:latest

# To login the docker container and check the stdout_log
docker exec -it arpa-node sh
/ # vi /var/log/randcast_node_client.log
/ # exit

# To check the node.log on the host machine
vi log/node.log
```

- Warning:

  - Please wait about 1 minute for the `node-client` to fully start, then [confirm the running status](#confirm-running-status).
  - Node registration will NOT be automatically performed on the first startup anymore since `v0.2.0`.
  - Please **DO NOT move or modify database file** after the first run, and **DO NOT modify node identity configuration arbitrarily**, otherwise errors will occur during runtime. (such as [DKG Key Mismatch](/docs//issue%20fix/dkg-public-key-mismatch.md))
  - It is recommended to **keep the nodes long-running**. Please DO NOT frequently start and stop, which may result in missing grouping or task events and cause unnecessary slashing. (see [Reactivate once Slashed](/docs//issue%20fix/reactivate-once-slashed.md))
  - Please ensure regular backup of the database file `./db/data.sqlite`.
  - It is recommended to observe and analyze the complete `node.log` on the host machine. If the container starts and stops every time using the `docker rm` command, the standard output log is incomplete.

- Note:
  - At present, we will collect data in log file `node.log` to locate and troubleshoot issues, but please note that the logs **WILL NOT** contain node private content or running environment metrics
  - To run multiple node clients on the same machine, you may need to change the path to "/<YOUR_EXPECTED_SUBFOLDER>" behind of the "db" or "log" directory.

### Register to ARPA Network:

1. Go to the log and search the log for the keyword "public_key"(or "DKGKeyGenerated") and copy the DKG public key value.
2. Call `nodeRegister` method of `NodeRegistry` contract, for your reference:

- Currently, sending transactions through Etherscan may encounter issues with incorrect number of parameters. We recommend using [Foundry Cast](https://book.getfoundry.sh/reference/cast/cast-send) or other programming language libraries for registration.
- [NodeRegistry Contract](https://github.com/ARPA-Network/BLS-TSS-Network/blob/0732850fe39f869a7dea899e445dfe6332462ab7/contracts/src/interfaces/INodeRegistry.sol#L25)
- [Generate the EIP1271 Operator signature](#generate-the-eip1271-operator-signature-for-avs-registration-with-your-asset-account)
- Contract address listed in our [Official Document](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters)
- Example calldata should look like this

| Name                            | Type    | Data                                                                                                                                                                                                                                                               |
| ------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| dkgPublicKey                    | bytes   | 0x1b0eec420a74cdd1fdbf7ed48c03b70ddfd2507f14aa54a2bbdefe6a92da93b72832d11c106610b175cc04b3fd7d5b6b869d0571d80b5c240b9213d8e1a1d0b7030f40df5cd261d4497ba15de4bb1769ec45c0ecc69a477fbecfc82c96668916042a01a85083f80174c55b5d6959ba3c29c589676978a37d4f6646c4bb6da116 |
| isEigenlayerNode                | bool    | true                                                                                                                                                                                                                                                               |
| assetAccountAddress             | address | 0xE473d420C10a4e12c90408198B750Bf38a92Fa16                                                                                                                                                                                                                         |
| assetAccountSignature.signature | bytes   | 0x444c3c9a6d487299162b02ac7e705ba533bc03445eda8d2e4f498bf430cbe21421a3c8933b0fa08e5fb43ee2a38896028694accebd0f77620eebf9bb93a3d4fc1c                                                                                                                               |
| assetAccountSignature.salt      | bytes32 | 0x7fe94cad56d5aaeb5921b08ca90668654210fde42fed0c9507e8b5d796491bfc                                                                                                                                                                                                 |
| assetAccountSignature.expiry    | uint256 | 1717429132                                                                                                                                                                                                                                                         |

Note: After successful registration transaction, it is normal to NOT have instant log output in node.log since it may be waiting for other nodes to group.

#### Generate the EIP1271 Operator signature for AVS registration with your `Asset` Account

- The signature we need is the same as the signature needed by the `registerOperatorToAVS` method in the `AVSDirectory` contract, used for proving that the `Node` account has the right to register the corresponding operator to our ARPA Network AVS.
- For more information about the format of the signature, please refer to [Eigenlayer AVSDirectory Doc](https://github.com/Layr-Labs/eigenlayer-contracts/blob/dev/docs/core/AVSDirectory.md#registeroperatortoavs).
- If you are a smart-contract operator, please upgrade the contract to generate / verify the signature. If you are an EOA operator, we have prepared a script that you can modify the parameters to run directly, or refer to the content to obtain the `digest_hash` of the signature and sign it with your operator account.
  - [signature-generation-holesky.sh](/docs/signature-generation-holesky.sh)
  - [signature-generation-mainnet.sh](/docs/signature-generation-mainnet.sh)
- Our AVS contract is named `ServiceManager` and the address is listed in our [Official Document](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters).

### Confirm Running Status

- New logs are generated after the `node-client` starts up.
- The following ports should be listening (according to your config). You can try to access those ports from external environments (such as telnet)
  - node_advertised_committer_rpc_endpoint: "<EXTERNAL_IP>:50061"
  - node_management_rpc_endpoint: "0.0.0.0:50091"
  - node_statistics_http_endpoint: "0.0.0.0:50081"
- Under certain conditions, the `node.log` located in `/db` directory should contain following logs with corresponding`log_type`:
  - After the successful DKG process `DKGGroupingAvailable`
  - After receiving any task `TaskReceived`
  - After working as normal node `PartialSignatureFinished` and `PartialSignatureSent`
  - After working as committer node `AggregatedSignatureFinished` and `FulfillmentFinished`
- Error logs do not necessarily represent irreversible errors. If you find that the error logs have grown significantly in a short period of time, please contact us.

### Log Off Node Account by Asset Account:

When the `Node` account needs to be replaced, make sure the `Node` account is in a non-working state (exited or slashed), then you can use the `Asset` account to call the `nodeLogOff` method of `NodeRegistry` contract, to reset the binding relationship between the `Asset` account and the `Node` account.

Afterwards, you can register a new `Node` account by calling `nodeRegister` method of `NodeRegistry` contract, to update the binding relationship between the `Asset` account and the `Node` account.

The old `Node` account **cannot** be activated or bound again. If you still hold its private key, you can retrieve the accumulated rewards through `nodeWithdraw` from the `NodeRegistry` contract.

### Exit Node from ARPA Network:

When the `Node` account needs to be exited from the ARPA Network, you can call the `nodeQuit` method of `NodeRegistry` contract by the `Node` account.

If the gas estimation fails, this is usually because the node is still in an unfinished DKG grouping process. Please check your `node.log` and find the latest log with "DKG grouping task received." message, wait for 40 blocks after the `assignment_block_height`, and then try again.

If it still fails, please call the `postProcessDkg` method of `Controller` contract by the `Node` account, with `group_index` and `epoch` found in the `node.log` above. This usually does not happen because it is automatically called in the `node-client` by all group members, and the successful caller is incentivized financially. If the problem persists, please contact us.

Afterwards, the operator is also deregistered from our AVS, and you can

- activate the node, make it groupable and workable again by calling `nodeActivate` method of `NodeRegistry` contract.
- change the DKG public key by calling `changeDkgPublicKey` method of `NodeRegistry` contract.
- reset the binding relationship between the `Asset` account and the `Node` account by calling `nodeLogOff` method of `NodeRegistry` contract by the `Asset` account, and then register a new `Node` account again by calling `nodeRegister` method of `NodeRegistry` contract.

## Troubleshooting

Known issues fix:

- [DKG Key Mismatch](/docs//issue%20fix/dkg-public-key-mismatch.md)
- [Reactivate once Slashed](/docs//issue%20fix/reactivate-once-slashed.md)

## **Reference**

- GitHub Repositories
  - [BLS-TSS-Network](https://github.com/ARPA-Network/BLS-TSS-Network)
  - [Arpa Node](https://github.com/ARPA-Network/BLS-TSS-Network/tree/main/crates/arpa-node)
- [ARPA Network Gitbook](https://docs.arpanetwork.io/)
- [ARPA Official Website](https://www.arpanetwork.io/en-US)
- [Contract Addresses](https://docs.arpanetwork.io/randcast/supported-networks-and-parameters)

## Update Log

- 06/03/2024
  - Fixed known bugs
  - Separated operator and node client private keys for EigenLayer users
  - Released new version (0.2.0)
- 05/24/2024
  - Mainnet published
  - Added log collection component (will not collect sensitive data)
- 04/17/2024
  - Testnet published
