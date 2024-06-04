# Node Setup Instruction for Eigenlayer Operator

### **Node**

The ARPA BLS-TSS Network consists of multiple groups of nodes. Within a group, each node is responsible for completing a BLS task (generating a BLS-TSS signature jointly with the other nodes of the group).

### **Prerequisites**

- **Minimum Hardware Requirements**
    
    Using AWS EC2 as an example, to ensure a steady performance, each node should be hosted on a virtual instance that meets the following specs:
    
    **t2.small (~$23/month)**
    
    - **1** vCPU
    - **2G** Memory
    - **30G** Storage
- [Docker](https://www.docker.com/get-started/)
- Externally accessible IP and ports

### **EL Operator Setup**

Follow the [official document](https://docs.eigenlayer.xyz/eigenlayer/operator-guides/operator-installation) to install and set up the operator accordingly.

After this, you should have:

- mainnet private key / address setup
- docker installed

### **Staking and Whitelist**

Due to the fact that Eigenlayer in the M2 phase does not support slashing, we collaborate with operators through a whitelist approach. Currently, there are no staking requirements set.

Nodes on the whitelist can join the ARPA Network and earn rewards by correctly performing the BLS tasks.

### **Reward**

Besides the cooperation agreement, there are three types of rewards a node can earn through smart contract if acted responsibly in a timely manner:

- Randomness submission reward
    - 1 ARPA each time a node successfully submits a randomness result.
- BLS-TSS task reward
    - 1 ARPA each time a node completes a BLS-TSS task.
- DKG post-process reward
    - 60 ARPA each time a node completes the DKG post-proccess task during node grouping.

### **Balance**

For gas fee of grouping operations and task submission, please ensure you have a sufficient ETH balance ready on all the L1/L2s that you need to support, which currently are:

- Testnet
    - ETH Holesky (17000)
    - Redstone Garnet (17069)
- Mainnet
    - ETH Mainnet (1)
    - OP Mainnet (10)
    - Base Mainnet (8453)
    - Redstone Mainnet (690)

***Note:*** The gas cost of fulfilling tasks(the signature verification and callback function) is directly paid by the committer node and then reimbursed by the requesting user. *N*ode operator can retrieve both the prepaid ETH and extra ARPA reward at any time by calling the `nodeWithdraw` method from the `NodeRegistry` contract on L1, or `ControllerOracle` contract on L2s.

### **Setup Steps**

1. Build a config (yml file) in your running environment
    
    Please copy the template below, and to change:
    
    - **provider_endpoint (Necessary, both main chain and relayed chains)**
    - **account (Necessary)**
    - node_management_rpc_endpoint, node_management_rpc_token (Optional)
    - listeners and time_limits (Optional and please modify carefully if needed)
    
    Please refer to [here](https://github.com/ARPA-Network/BLS-TSS-Network/tree/main/crates/arpa-node#node-config) for detailed instructions.
    
    ***Note:*** 
    
    - The contract addresses in the following example are currently the latest available.
    - We recommend setup account by keystore or hdwallet. Please **DO NOT use comments** in the config file.
    - Please confirm that the `node_advertised_committer_rpc_endpoint` can be accessed by the external network.
        - example:
        
        ```jsx
        account:
          private_key: "<YOUR_PRIVATE_KEY>"
          
        	# keystore:
          #   password: env
          #   path: test.keystore
        
          # hdwallet:
          #   mnemonic: env
          #   path: "m/44'/60'/0'/0"
          #   index: 0
          #   passphrase: "custom_password"
        ```
        
    
    **Testnet config.yml example**
    
    [config for Testnet](/docs/config.holesky.yaml)
    
    **Mainnet config.yml example**
    
    [config for Mainnet](/docs/config.mainnet.yaml)
    
2. Run below commands to register and start the node-client:

```bash
#!/bin/bash
cd <YOUR_ARPA_NETWORK_ROOT_DIRECTORY>

# Suppose you have a config.yml here
# Use `node-config-checker` to make the integrity check
# It will print out the address of the account provided in the configuration file, 
# otherwise the error reason will be printed
docker run -v ./config.yml:/app/config.yml ghcr.io/arpa-network/node-config-checker:latest "node-config-checker -c /app/config.yml"

# Create the necessary directories
mkdir db
mkdir log

# Create the config.yml file and fill in config details in step #1

# Pull the latest Docker image
docker pull ghcr.io/arpa-network/node-client:latest

# Run the Docker container
docker run -d \
--name arpa-node \
-v ./config.yml:/app/config.yml \
-v ./db:/app/db\
-v ./log:/app/log \
--network=host \
ghcr.io/arpa-network/node-client:latest

# To login the docker container and check the stdout_log
docker exec -it arpa-node sh
/ # vi /var/log/randcast_node_client.log
/ # exit

# To check the node.log on the host machine
vi log/node.log
```

- Note:
    - Node registration will be automatically performed on the first startup. Please **DO NOT move or modify database file** after the first run, and **DO NOT modify node identity configuration arbitrarily**, otherwise errors will occur during runtime.
    - It is recommended to **keep the nodes long-running**. Please avoid frequent start and stop, which may result in missing grouping or task events and causing unnecessary slashing.
    - Please ensure regular backup of the database file `./db/data.sqlite`.
    - It is recommended to observe and analyze the complete `node.log` on the host machine. If the container starts and stops every time using the `docker rm` command, the standard output log is incomplete.
    - If you run multiple node clients on the same machine, you may need to run it like below to add “/<YOUR_EXPECTED_SUBFOLDER>” behind of the “db” or “log” directory (highlighted in blue). Note that please **DO NOT** change the “app/db” or “app/log” part as they are relative path in docker instance.
    
    ```bash
    docker run -d \
    --name arpa-node-1 \
    -v ./config_1.yml:/app/config.yml \
    -v ./db/1:/app/db\
    -v ./log/1:/app/log \
    --network=host \
    ghcr.io/arpa-network/node-client:latest
    ```
    
    - At present, we will collect data in log file `node.log` to locate and troubleshoot issues, but please be aware that the logs **WILL NOT** contain node private content or running environment metrics
    - For existing user, to upgrade to v0.2.0, please follow [upgrade instruction ](/docs/upgrade-instruction.md)
1. Observe to confirm running status
    - The following ports should be on listening(according to your config)
        - node_committer_rpc_endpoint: "0.0.0.0:50061"
        - node_management_rpc_endpoint: "0.0.0.0:50091"
        - node_statistics_http_endpoint: "0.0.0.0:50081"
    - The `node.log` under `/db` should have following logs with `log_type` under certain conditions:
        - After the first run  `NodeRegistered`
        - After automatic activation from being slashed unexpectedly  `NodeActivated`
        - After the successful DKG process  `DKGGroupingAvailable`
        - After receiving any task  `TaskReceived`
        - After working as normal node  `PartialSignatureFinished` and `PartialSignatureSent`
        - After working as committer node  `AggregatedSignatureFinished` and `FulfillmentFinished`
    - Error logs do not necessarily represent irreversible errors. If you find that the error logs have grown significantly in a short period of time, please contact us.

### **Reference**

- GitHub Repositories
    - [BLS-TSS-Network](https://github.com/ARPA-Network/BLS-TSS-Network)
    - [Arpa Node](https://github.com/ARPA-Network/BLS-TSS-Network/tree/main/crates/arpa-node)
- [ARPA Network Gitbook](https://docs.arpanetwork.io/)
- [ARPA Official Website](https://www.arpanetwork.io/en-US)

### Update Log

- 06/03/20234
    - Fixed known bugs
    - Separated operator and node client private keys for EigenLayer users
    - Release new version (0.2.0)
- 05/24/2024
    - Mainnet published
    - Added log collection component (will not collect sensitive data)
- 04/17/2024
    - Testnet published