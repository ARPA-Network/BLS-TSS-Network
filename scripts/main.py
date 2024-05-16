import ast
import binascii
import json
import os
import subprocess
import sys
import time
from pprint import pprint

import ruamel.yaml
import termcolor
from dotenv import dotenv_values, get_key, load_dotenv, set_key
from pk_util import print_keypair

####################
# Global Variables #
####################


# prep directories
SCRIPT_DIR = os.getcwd()
ROOT_DIR = os.path.abspath(os.path.join(SCRIPT_DIR, os.pardir))
ARPA_NODE_DIR = os.path.join(ROOT_DIR, "crates/arpa-node")
ARPA_NODE_CONFIG_DIR = os.path.join(ARPA_NODE_DIR, "test/conf")
CONTRACTS_DIR = os.path.join(ROOT_DIR, "contracts")
ENV_EXAMPLE_PATH = os.path.join(CONTRACTS_DIR, ".env.example")
ENV_PATH = os.path.join(CONTRACTS_DIR, ".env")
ADDRESSES_JSON_PATH = os.path.join(SCRIPT_DIR, "addresses.json")
NODE_CLIENT_DIR = os.path.join(ROOT_DIR, "docker/node-client")
NODE_CLIENT_BINARY_DIR = os.path.join(ROOT_DIR, "target/release")


# RPC INFO
L2_CHAIN_ID = get_key(ENV_PATH, "OP_CHAIN_ID")
L2_RPC = get_key(ENV_PATH, "OP_RPC")
L2_WS_RPC = get_key(ENV_PATH, "OP_WS_RPC")
L1_CHAIN_ID = get_key(ENV_PATH, "L1_CHAIN_ID")
L1_RPC = get_key(ENV_PATH, "L1_RPC")
L1_WS_RPC = get_key(ENV_PATH, "L1_WS_RPC")

# Deployment flags
LOCAL_TEST = get_key(ENV_PATH, "LOCAL_TEST").lower() == "true"


ARPA_EXISTS = (
    get_key(ENV_PATH, "ARPA_EXISTS").lower() == "true"
)  # bool True if ARPA_EXISTS is true in .env
L2_ONLY = (
    get_key(ENV_PATH, "L2_ONLY").lower() == "true"
)  # bool True if L2_ONLY is true in .env
BASE_DEPLOYMENT = (
    get_key(ENV_PATH, "OP_CHAIN_ID") == "8453"
    or get_key(ENV_PATH, "OP_CHAIN_ID") == "84531"
)
REDSTONE_DEPLOYMENT = get_key(ENV_PATH, "OP_CHAIN_ID") == "17001"
LOOTCHAIN_DEPLOYMENT = get_key(ENV_PATH, "OP_CHAIN_ID") == "9088912"

# Admin Private Key used to Relay Groups manually during L2_ONLY deployment
ADMIN_PRIVATE_KEY = get_key(ENV_PATH, "ADMIN_PRIVATE_KEY")
VERBOSE_OUTPUT = get_key(ENV_PATH, "VERBOSE_OUTPUT").lower() == "true"
HIDE_OUTPUT = not VERBOSE_OUTPUT  # if verbose_output = false, hide_output = true

# Existing L1 Addresses
EXISTING_OP_ARPA_ADDRESS = get_key(ENV_PATH, "EXISTING_OP_ARPA_ADDRESS")
EXISTING_L1_ARPA_ADDRESS = get_key(ENV_PATH, "EXISTING_L1_ARPA_ADDRESS")
EXISTING_L1_STAKING_ADDRESS = get_key(ENV_PATH, "EXISTING_L1_STAKING_ADDRESS")
EXISTING_L1_SERVICE_MANAGER_ADDRESS = get_key(
    ENV_PATH, "EXISTING_L1_SERVICE_MANAGER_ADDRESS"
)
EXISTING_L1_CONTROLLER_ADDRESS = get_key(ENV_PATH, "EXISTING_L1_CONTROLLER_ADDRESS")
EXISTING_L1_ADAPTER_ADDRESS = get_key(ENV_PATH, "EXISTING_L1_ADAPTER_ADDRESS")
EXISTING_L1_CONTROLLER_RELAYER = get_key(ENV_PATH, "EXISTING_L1_CONTROLLER_RELAYER")
OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS = get_key(
    ENV_PATH, "OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS"
)

# Staking Environment Variables
NODE_PRIVATE_KEY_COUNT = get_key(ENV_PATH, "NODE_PRIVATE_KEY_COUNT")

print(f"L1_CHAIN_ID: {L1_CHAIN_ID}")
print(f"L1_RPC: {L1_RPC}")
print(f"L2_CHAIN_ID: {L2_CHAIN_ID}")
print(f"L2_RPC: {L2_RPC}")
print(f"ARPA_EXISTS: {ARPA_EXISTS}")
print(f"L2_ONLY: {L2_ONLY}")

OP_CONTRACTS_DEPLOYMENT_BROADCAST_PATH = os.path.join(
    CONTRACTS_DIR,
    "broadcast",
    "OPControllerOracleLocalTest.s.sol",
    L2_CHAIN_ID,
    "run-latest.json",
)
L1_CONTRACTS_DEPLOYMENT_BROADCAST_PATH = os.path.join(
    CONTRACTS_DIR,
    "broadcast",
    "ControllerLocalTest.s.sol",
    L1_CHAIN_ID,
    "run-latest.json",
)
CONTRACTS_DEPLOYMENT_ADDRESSES_PATH = os.path.join(
    CONTRACTS_DIR, get_key(ENV_PATH, "DEPLOYMENT_ADDRESSES_PATH")
)


# ! New - Generalize by making a general "OPStackChainMessenger contract" and "CreateAndSetChainMessengerScript" script
CREATE_AND_SET_OP_STACK_CHAIN_MESSENGER_BROADCAST_PATH = os.path.join(
    CONTRACTS_DIR,
    "broadcast",
    "CreateAndSetOPStackChainMessenger.s.sol",
    L1_CHAIN_ID,
    "run-latest.json",
)


NODE_CLIENT_RELEASE_BINARY_PATH = os.path.join(ROOT_DIR, "target/release/node-client")
NODE_CLIENT_DEBUG_BINARY_PATH = os.path.join(ROOT_DIR, "target/debug/node-client")

L1_TRANSACTION_PRIORITY_GAS_PRICE = get_key(
    ENV_PATH, "L1_TRANSACTION_PRIORITY_GAS_PRICE"
)
L2_TRANSACTION_PRIORITY_GAS_PRICE = get_key(
    ENV_PATH, "L2_TRANSACTION_PRIORITY_GAS_PRICE"
)


def cprint(text: str, color: str = "green"):
    """Prints text in color

    Args:
        text (str): the text to be printed
        color (str): the color to be used
    """
    termcolor.cprint(text, color)


def get_addresses_from_broadcast_json(path: str) -> dict:
    """
    Given a path to the broadcast json file, return a dictionary of contract names to addresses
    """

    # Initialize an empty dictionary
    contracts_dict = {}

    # Open the json file
    with open(path, "r") as read_file:
        data = json.load(read_file)  # Load the json contents
        transactions = data.get(
            "transactions", []
        )  # Get the list of transactions or an empty list if "transactions" key does not exist

        # Loop through each transaction
        for transaction in transactions:
            contract_name = transaction.get("contractName")
            contract_address = transaction.get("contractAddress")

            # If both contractName and contractAddress exists, add to dictionary
            if contract_name and contract_address and contract_name != "ERC1967Proxy":
                contracts_dict[contract_name] = contract_address

    return contracts_dict


def get_addresses_from_json(path: str) -> dict:
    """
    Given a path to a json file, return a dictionary of contract names to addresses
    """
    with open(path, "r") as f:
        addresses = json.load(f)
    return addresses


def get_l1_addresses():
    l1_controller_addresses = {}
    if L2_ONLY:
        l1_controller_addresses["Arpa"] = EXISTING_L1_ARPA_ADDRESS
        l1_controller_addresses["Staking"] = EXISTING_L1_STAKING_ADDRESS
        l1_controller_addresses["ServiceManager"] = EXISTING_L1_SERVICE_MANAGER_ADDRESS
        l1_controller_addresses["Controller"] = EXISTING_L1_CONTROLLER_ADDRESS
        l1_controller_addresses["Adapter"] = EXISTING_L1_ADAPTER_ADDRESS
        l1_controller_addresses["ControllerRelayer"] = EXISTING_L1_CONTROLLER_RELAYER

        l1_addresses = {**l1_controller_addresses}

        l1_chain_op_stack_messenger_addresses = get_addresses_from_broadcast_json(
            CREATE_AND_SET_OP_STACK_CHAIN_MESSENGER_BROADCAST_PATH
        )
        l1_addresses.update(l1_chain_op_stack_messenger_addresses)
    else:
        l1_addresses = get_addresses_from_broadcast_json(
            L1_CONTRACTS_DEPLOYMENT_BROADCAST_PATH
        )
        l1_addresses.update(
            get_addresses_from_json(CONTRACTS_DEPLOYMENT_ADDRESSES_PATH)["L1"]
        )
        l1_chain_op_stack_messenger_addresses = get_addresses_from_broadcast_json(
            CREATE_AND_SET_OP_STACK_CHAIN_MESSENGER_BROADCAST_PATH
        )
        l1_addresses.update(l1_chain_op_stack_messenger_addresses)
        if ARPA_EXISTS:
            l1_addresses["Arpa"] = EXISTING_L1_ARPA_ADDRESS
    return l1_addresses


def get_l2_addresses():
    l2_addresses = get_addresses_from_json(CONTRACTS_DEPLOYMENT_ADDRESSES_PATH)["L2"]
    if ARPA_EXISTS:
        l2_addresses["Arpa"] = EXISTING_OP_ARPA_ADDRESS
    return l2_addresses


def run_command(
    cmd: list,
    check=True,
    shell=False,
    cwd=None,
    env=None,
    capture_output=False,  # default to not capturing output (printing directly to stdout).
    # Set to true if you want to suppress output / need to set output to a variable.
    text=False,
):
    """
    Run a command in a subprocess, raising an exception if it fails.
    """
    env = env if env else {}
    return subprocess.run(
        cmd,
        check=check,
        shell=shell,
        env={**os.environ, **env},
        cwd=cwd,
        capture_output=capture_output,
        text=text,
    )


def wait_command(
    cmd: list,
    shell=False,
    env=None,
    cwd=None,
    wait_time=1,
    max_attempts=1,
    fail_value=None,
    success_value=None,
) -> str:
    """Checks for the success of a command after a set interval.
        Returns the stdout if successful or None if it fails.

    Args:
        cmd (List[str]): the command to be run
        wait_time (int): the time to wait between attempts
        max_attempts (int): the maximum number of attempts
        shell (bool): whether to use shell or not
        env (dict): the environment variables dictionary
        cwd (str): the current working directory
        fail_value (str): value that when provided, must not match the output in order to succeed
        success_value (str): value that when provided, must match the output in order to succeed

    Returns:
        str: stdout if the command finishes successfully, None otherwise
    """
    fail_counter = 0

    while True:
        command_output = run_command(
            cmd,
            shell=shell,
            env=env,
            check=False,
            cwd=cwd,
            capture_output=True,  # ALWAYS TRUE as output is needed to check for success
            text=True,
        )
        # If command_output.stdout is not None, strip it
        stdout = command_output.stdout.strip() if command_output.stdout else None

        # # Debugging
        # print("command_output.returncode: ", command_output.returncode)
        # print("command_output.stdout: ", command_output.stdout)
        # print("stdout: ", stdout)
        # print("fail_value: ", fail_value)
        # print("success_value: ", success_value)

        # Judge whether the command is successful
        if (
            command_output.returncode == 0  # If the command is successful
            and stdout is not None  # If stdout is not None
        ):
            # If neither success_value or fail_value is set, return stdout
            if success_value is None and fail_value is None:
                return stdout
            # If success_value is set, but fail_value is not, return stdout if stdout == success_value
            if success_value is not None and fail_value is None:
                if stdout == success_value:
                    return stdout
            # If fail_value is set, but success_value is not, return stdout if stdout != fail_value
            if success_value is None and fail_value is not None:
                if stdout != fail_value:
                    return stdout
            # If both success_value and fail_value are set, return stdout if stdout == success_value and stdout != fail_value
            if success_value is not None and fail_value is not None:
                if (stdout == success_value) and (stdout != fail_value):
                    return stdout

        # If the command fails, print a dot and increment the fail_counter
        print(".", end="", flush=True)
        fail_counter += 1

        # If the command fails for max_attempts consecutive times, return None
        if fail_counter >= max_attempts:
            print(
                f"\nError: Command did not finish after {wait_time*max_attempts} seconds. Exiting..."
            )
            return None
            # sys.exit(1)

        # Wait for wait_time seconds before trying again
        time.sleep(wait_time)


def deploy_contracts():
    ##################################
    ###### Contract Deployment #######
    ##################################

    # 1. Copy .env.example to .env, and load .env file for editing
    # run_command(["cp", ENV_EXAMPLE_PATH, ENV_PATH])

    # 2. Deploy L2 OPControllerOracleLocalTest contracts
    print("Running Solidity Script: OPControllerOracleLocalTest on L2...")
    cmd = f"forge script script/OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript --fork-url {L2_RPC} --broadcast --priority-gas-price {L2_TRANSACTION_PRIORITY_GAS_PRICE}"
    if LOOTCHAIN_DEPLOYMENT:
        cmd = cmd + " --slow --legacy"

    cprint(cmd)
    run_command(
        [cmd], env={}, cwd=CONTRACTS_DIR, capture_output=HIDE_OUTPUT, shell=True
    )
    # get L2 contract addresses from broadcast and update .env file
    l2_addresses = get_addresses_from_json(CONTRACTS_DEPLOYMENT_ADDRESSES_PATH)["L2"]
    if ARPA_EXISTS:
        l2_addresses["Arpa"] = EXISTING_OP_ARPA_ADDRESS

    set_key(ENV_PATH, "OP_ADAPTER_ADDRESS", l2_addresses["Adapter"])
    set_key(ENV_PATH, "OP_ARPA_ADDRESS", l2_addresses["Arpa"])
    set_key(ENV_PATH, "OP_CONTROLLER_ORACLE_ADDRESS", l2_addresses["ControllerOracle"])

    if not L2_ONLY:  # l2_only = false
        # 3. Deploy L1 ControllerLocalTest contracts
        print("Running Solidity Script: ControllerLocalTest on L1...")
        cmd = f"forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url {L1_RPC} --broadcast --priority-gas-price {L1_TRANSACTION_PRIORITY_GAS_PRICE}"
        cprint(cmd)
        run_command(
            [cmd],
            env={},
            cwd=CONTRACTS_DIR,
            capture_output=HIDE_OUTPUT,
            shell=True,
        )
        # get L1 contract addresses from broadcast and update .env file
        l1_addresses = get_addresses_from_json(CONTRACTS_DEPLOYMENT_ADDRESSES_PATH)[
            "L1"
        ]
        if ARPA_EXISTS:
            l1_addresses["Arpa"] = EXISTING_L1_ARPA_ADDRESS

        set_key(ENV_PATH, "ARPA_ADDRESS", l1_addresses["Arpa"])
        set_key(ENV_PATH, "CONTROLLER_ADDRESS", l1_addresses["Controller"])
        set_key(ENV_PATH, "ADAPTER_ADDRESS", l1_addresses["Adapter"])
        set_key(ENV_PATH, "NODE_REGISTRY_ADDRESS", l1_addresses["NodeRegistry"])
        set_key(ENV_PATH, "SERVICE_MANAGER_ADDRESS", l1_addresses["ServiceManager"])
        set_key(ENV_PATH, "STAKING_ADDRESS", l1_addresses["Staking"])

        l1_controller_relayer = l1_addresses["ControllerRelayer"]

    else:  # l2_only == True
        l1_controller_addresses = {}
        l1_controller_addresses["Arpa"] = EXISTING_L1_ARPA_ADDRESS
        l1_controller_addresses["Staking"] = EXISTING_L1_STAKING_ADDRESS
        l1_controller_addresses["ServiceManager"] = EXISTING_L1_SERVICE_MANAGER_ADDRESS
        l1_controller_addresses["Controller"] = EXISTING_L1_CONTROLLER_ADDRESS
        l1_controller_addresses["Adapter"] = EXISTING_L1_ADAPTER_ADDRESS

        l1_addresses = {**l1_controller_addresses}
        set_key(ENV_PATH, "ARPA_ADDRESS", l1_addresses["Arpa"])
        set_key(ENV_PATH, "CONTROLLER_ADDRESS", l1_addresses["Controller"])
        set_key(ENV_PATH, "ADAPTER_ADDRESS", l1_addresses["Adapter"])
        set_key(ENV_PATH, "SERVICE_MANAGER_ADDRESS", l1_addresses["ServiceManager"])
        set_key(ENV_PATH, "STAKING_ADDRESS", l1_addresses["Staking"])
        l1_controller_relayer = EXISTING_L1_CONTROLLER_RELAYER

    # Deploy CreateAndSetChainMessenger script
    #! Generalize by making a general "OPStackChainMessenger contract" and "CreateAndSetChainMessengerScript" script

    # ! New
    print("Running Solidity Script: CreateAndSetOPStackChainMessenger on L1...")
    cmd = f"forge script script/CreateAndSetOPStackChainMessenger.s.sol:CreateAndSetOPStackChainMessengerScript --fork-url {L1_RPC} --broadcast --priority-gas-price {L1_TRANSACTION_PRIORITY_GAS_PRICE}"
    cprint(cmd)

    run_command(
        [cmd],
        env={
            "OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS": OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS,
            "OP_CONTROLLER_ORACLE_ADDRESS": l2_addresses["ControllerOracle"],
            "EXISTING_L1_CONTROLLER_RELAYER": l1_controller_relayer,
        },
        cwd=CONTRACTS_DIR,
        capture_output=HIDE_OUTPUT,
        shell=True,
    )

    # ! New
    l1_chain_op_stack_messenger_addresses = get_addresses_from_broadcast_json(
        CREATE_AND_SET_OP_STACK_CHAIN_MESSENGER_BROADCAST_PATH
    )
    l1_addresses.update(l1_chain_op_stack_messenger_addresses)

    set_key(
        ENV_PATH, "L1_CHAIN_MESSENGER_ADDRESS", l1_addresses["OPStackChainMessenger"]
    )

    # 4. deploy remaining contracts (Controller Oracle Init, StakeNodeLocalTest)
    print(
        "Running Solidity Script: OPControllerOracleInitializationLocalTestScript on L2..."
    )
    cmd = f"forge script script/OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript --fork-url {L2_RPC} --broadcast --priority-gas-price {L2_TRANSACTION_PRIORITY_GAS_PRICE}"
    if LOOTCHAIN_DEPLOYMENT:
        cmd = cmd + " --slow --legacy"
    cprint(cmd)
    run_command(
        [cmd],
        env={
            "OP_ADAPTER_ADDRESS": l2_addresses["Adapter"],
            "OP_ARPA_ADDRESS": l2_addresses["Arpa"],
            "OP_CONTROLLER_ORACLE_ADDRESS": l2_addresses["ControllerOracle"],
            "L1_CHAIN_MESSENGER_ADDRESS": l1_addresses["OPStackChainMessenger"],
        },
        cwd=CONTRACTS_DIR,
        capture_output=HIDE_OUTPUT,
        shell=True,
    )

    if not L2_ONLY:  # l2_only == False
        if NODE_PRIVATE_KEY_COUNT != "0":
            print("Running Solidity Script: InitStakingLocalTestScript on L1...")
            cmd = f"forge script script/InitStakingLocalTest.s.sol:InitStakingLocalTestScript --fork-url {L1_RPC} --broadcast -g 150 --priority-gas-price {L1_TRANSACTION_PRIORITY_GAS_PRICE}"
            cprint(cmd)
            run_command(
                [cmd],
                env={
                    "ARPA_ADDRESS": l1_addresses["Arpa"],
                    "STAKING_ADDRESS": l1_addresses["Staking"],
                },
                cwd=CONTRACTS_DIR,
                capture_output=HIDE_OUTPUT,
                shell=True,
            )
            for i in range(1, int(NODE_PRIVATE_KEY_COUNT) + 1):
                print("Running Solidity Script: StakeNodeLocalTestScript on L1...")
                cmd = f"forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url {L1_RPC} --broadcast -g 150 --priority-gas-price {L1_TRANSACTION_PRIORITY_GAS_PRICE}"
                cprint(cmd)
                run_command(
                    [cmd],
                    env={
                        "ARPA_ADDRESS": l1_addresses["Arpa"],
                        "STAKING_ADDRESS": l1_addresses["Staking"],
                        "ADAPTER_ADDRESS": l1_addresses["Adapter"],
                        "NODE_PRIVATE_KEY_OFFSET": str(i),
                    },
                    cwd=CONTRACTS_DIR,
                    capture_output=HIDE_OUTPUT,
                    shell=True,
                )
                time.sleep(3)  # avoid replacement transaction underpriced

    else:  # l2_only == True
        # determine number of available groups and relay groups
        print(
            "Determining number of available groups and relayinig those groups from L1 to L2..."
        )
        relay_groups(l2_addresses["ControllerOracle"])

    # Print addresses to addresses.json in the current directory
    print_addresses()


def print_node_key_info():
    mnemonic = get_key(ENV_PATH, "STAKING_NODES_MNEMONIC")
    for index in [1, 2, 3, 4, 5]:
        private_key, public_key = print_keypair(mnemonic, index)
        print("\nIndex: {}".format(index))
        print("Public key: {}".format(public_key))
        print("Private key: {}".format(private_key))


def deploy_nodes():  # ! Deploy Nodes
    with open(ADDRESSES_JSON_PATH, "r") as f:
        addresses = json.load(f)
        l1_addresses = addresses["L1 Addresses"]
        l2_addresses = addresses["L2 Addresses"]

    ######################################
    ###### ARPA Network Deployment #######
    ######################################

    # delete all config files in NODE_CLIENT_DIR
    os.system(f"rm -f {NODE_CLIENT_DIR}/config_*")
    config_template_file = os.path.join(NODE_CLIENT_DIR, "template.yml")

    # get node private keys from .env
    node_private_keys = []
    i = 1
    while True:
        key_name = f"NODE_PRIVATE_KEY_{i}"
        key_value = get_key(ENV_PATH, key_name)
        if key_value is None:
            break
        # strip the prepending 0x from the key
        if key_value.startswith("0x"):
            key_value = key_value[2:]
        node_private_keys.append(key_value)
        # copy template.yml to config_i.yml
        config_file = f"config_{i}.yml"
        os.system(f"cp {config_template_file} {NODE_CLIENT_DIR}/{config_file}")
        i += 1

    print(f"{len(node_private_keys)} private keys found in .env")

    node_private_key_count = get_key(ENV_PATH, "NODE_PRIVATE_KEY_COUNT")
    if len(node_private_keys) != int(node_private_key_count):
        cprint(
            f"WARNING: NODE_PRIVATE_KEY_COUNT in .env ({node_private_key_count}) does not match the number of private keys found ({len(node_private_keys)}). Exiting..."
        )

    # prep yaml writer
    yaml = ruamel.yaml.YAML()
    yaml.preserve_quotes = True  # preserves quotes
    yaml.indent(sequence=4, offset=2)  # set indentation

    # create config files for an arbitrary number of private_keys
    config_files = []  # used when deploying nodes later
    PORT_1 = 50061  # node rpc endpoint starting port
    PORT_2 = 50091  # node management rpc endpoint starting port
    PORT_3 = 50081  # node statistics http endpoint starting port
    for i, private_key in enumerate(node_private_keys):
        config_file = f"config_{i+1}.yml"
        config_files.append(config_file)
        file_path = os.path.join(NODE_CLIENT_DIR, config_file)
        with open(file_path, "r") as f:
            data = yaml.load(f)

        # set node commiter, node management and node statistics server endpoints # ! new
        data["node_advertised_committer_rpc_endpoint"] = f"0.0.0.0:{PORT_1 + i}"
        data["node_committer_rpc_endpoint"] = f"0.0.0.0:{PORT_1 + i}"
        data["node_management_rpc_endpoint"] = f"0.0.0.0:{PORT_2 + i}"
        data["node_statistics_http_endpoint"] = f"0.0.0.0:{PORT_3 + i}"

        # set is_eigenlayer
        data["is_eigenlayer"] = False

        # set node_id, data_path, and log_file_path
        data["data_path"] = f"./db/data{i+1}.sqlite"
        data["logger"]["log_file_path"] = f"log/{i+1}/"
        # L1
        data["adapter_address"] = l1_addresses["Adapter"]
        data["controller_address"] = l1_addresses["Controller"]
        data["controller_relayer_address"] = l1_addresses["ControllerRelayer"]
        # L2
        data["relayed_chains"][0]["controller_oracle_address"] = l2_addresses[
            "ControllerOracle"
        ]
        data["relayed_chains"][0]["adapter_address"] = l2_addresses["Adapter"]

        # update rpc endpoints
        data["provider_endpoint"] = L1_WS_RPC
        data["relayed_chains"][0]["provider_endpoint"] = L2_WS_RPC

        # Update Chain ID
        data["chain_id"] = int(L1_CHAIN_ID)
        data["relayed_chains"][0]["chain_id"] = int(L2_CHAIN_ID)

        # node private key
        data["account"]["private_key"] = private_key

        with open(file_path, "w") as f:
            yaml.dump(data, f)

        print(f"Node-client config file {config_file} created successfully!")

    # deploy nodes
    print("Starting randcast nodes...")
    PORT_1 = 50061
    PORT_2 = 50091
    for i, config_file in enumerate(config_files, start=1):
        print(f"Starting Node #{i} using: {config_file}!")
        if LOCAL_TEST:
            cmd = f"cargo run --bin node-client -- -c {NODE_CLIENT_DIR}/{config_file} > /dev/null 2>&1 &"
            # cmd = f"{NODE_CLIENT_DEBUG_BINARY_PATH} -c {NODE_CLIENT_DIR}/{config_file} > /dev/null 2>&1 &"  # debug binary

            # cmd = f"cargo run --release --bin node-client -- -c {NODE_CLIENT_DIR}/{config_file} > /dev/null 2>&1 &"
            # cmd = f"{NODE_CLIENT_RELEASE_BINARY_PATH} -c {NODE_CLIENT_DIR}/{config_file} > /dev/null 2>&1 &"  # release binary

        else:
            cmd = (
                f"docker run -d "
                f"--name node{i} "
                # f"-p {PORT_1 + i-1}:{PORT_1 + i-1} -p {PORT_2 + i-1}:{PORT_2 + i-1} "
                f"-v {ROOT_DIR}/docker/node-client/{config_file}:/app/config.yml "
                f"-v {ROOT_DIR}/docker/node-client/db:/app/db "
                f"-v {ROOT_DIR}/docker/node-client/log/{i}:/app/log/{i} "
                f"--network=host "
                f"ghcr.io/arpa-network/node-client:latest"
            )
        cprint(cmd)
        run_command(
            [cmd],
            cwd=NODE_CLIENT_DIR,
            shell=True,
        )

    # Wait for nodes to group
    if L2_ONLY:
        return  # no need to wait for nodes to group

    # wait for succesful grouping (fail after 1m without grouping)
    print("Waiting for nodes to group... ")
    time.sleep(5)  # wait for node.log file to be created
    cmd = f"cat {NODE_CLIENT_DIR}/log/1/node.log | grep 'available'"
    cprint(cmd)
    nodes_grouped = wait_command(
        [cmd],
        wait_time=12,
        max_attempts=60,  # fails on 50 sometimes, upped to 60.
        shell=True,
    )

    if nodes_grouped:
        print("\nNodes grouped succesfully!")
        print("Output:\n", nodes_grouped, "\n")
    else:
        print("Nodes failed to group!")
        # print out logs
        run_command(
            [
                f"cat {NODE_CLIENT_DIR}/log/1/node.log | tail",
            ],
            shell=True,
        )
        print("Quitting...")
        sys.exit(1)

    # Wait for DKG Proccess to Finish
    print(
        "Waiting for DKG Proccess to complete (group 0 coordinator should zero out)..."
    )
    # call controller.getCoordinator(). If it returns 0, we know dkg proccess finished and post proccess dkg has been called
    cmd = f"cast call {l1_addresses['Controller']} 'getCoordinator(uint256)' 0 --rpc-url {L1_RPC}"
    cprint(cmd)

    coordinator = wait_command(
        [cmd],
        wait_time=12,
        max_attempts=42,
        shell=True,
        success_value="0x0000000000000000000000000000000000000000000000000000000000000000",
    )
    print("\nDKG Proccess Completed Succesfully!")
    print(f"Coordinator Value: {coordinator}\n")


def get_last_randomness(address: str, rpc: str) -> str:
    last_randomness_l1 = wait_command(
        [f'cast call {address} "getLastRandomness()(uint256)" --rpc-url {rpc}'],
        wait_time=1,
        max_attempts=1,
        shell=True,
    ).strip()
    return last_randomness_l1


def test_request_randomness():

    # get l1_addresses and l2_addresses from addresses.json
    with open(ADDRESSES_JSON_PATH, "r") as f:
        addresses = json.load(f)
        l1_addresses = addresses["L1 Addresses"]
        l2_addresses = addresses["L2 Addresses"]
        # pprint(l1_addresses)
        # pprint(l2_addresses)

    # Check group state
    print("L1 Group Info:")
    cmd = f"cast call {l1_addresses['Controller']} \"getGroup(uint256)\" 0 --rpc-url {L1_RPC}"
    l1_group_info = run_command(
        [cmd],
        shell=True,
    )
    # print(l1_group_into)

    print("Waiting for group to relay from L1 to L2...")
    non_relayed_group = "0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"

    cmd = f"cast call {l2_addresses['ControllerOracle']} \"getGroup(uint256)\" 0 --rpc-url {L2_RPC}"

    cprint(cmd)
    l2_group_info = wait_command(
        [cmd],
        wait_time=15,
        max_attempts=10,
        fail_value=non_relayed_group,
        shell=True,
    )
    if l2_group_info:
        print(f"Group relayed from L1 to L2!")
        print("L2 Group Info:")
        print(l2_group_info)

    ############################################
    ###### L1 Request Randomness Testing #######
    ############################################

    # 1. Get L1 previous randomness
    l1_prev_randomness = get_last_randomness(l1_addresses["Adapter"], L1_RPC)

    # 2. Deploy L1 user contract and request randomness
    print("\nDeploying L1 user contract and requesting randomness...")
    cmd = f"forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url {L1_RPC} --broadcast --priority-gas-price {L1_TRANSACTION_PRIORITY_GAS_PRICE}"
    cprint(cmd)
    run_command(
        [cmd],
        env={
            "ADAPTER_ADDRESS": l1_addresses["Adapter"],
        },
        cwd=CONTRACTS_DIR,
        capture_output=HIDE_OUTPUT,
        shell=True,
    )
    l1_cur_randomness = get_last_randomness(l1_addresses["Adapter"], L1_RPC)

    # 3. Check if randomness is updated

    print("Waiting for randomness to be updated...")
    cmd = f'cast call {l1_addresses["Adapter"]} "getLastRandomness()(uint256)" --rpc-url {L1_RPC}'
    cprint(cmd)
    l1_cur_randomness = wait_command(
        [cmd],
        wait_time=15,
        max_attempts=20,
        fail_value=l1_prev_randomness,
        shell=True,
    )
    print(f"\nOld L1 randomness: {l1_prev_randomness}")
    print(f"New L1 randomness: {l1_cur_randomness}")
    print("L1 Requested Randomness succesfully!\n")

    ############################################
    ###### L2 Request Randomness Testing #######
    ############################################

    # 1. Get last randomness
    # get l2 previous randomness
    l2_prev_randomness = get_last_randomness(l2_addresses["Adapter"], L2_RPC)

    # 2. Deploy l2 user contract and request randomness
    print("Deploying l2 user contract and requesting randomness...")

    # forge script script/OPGetRandomNumberLocalTest.s.sol:OPGetRandomNumberLocalTestScript --fork-url http://localhost:9545 --broadcast
    cmd = f"forge script script/OPGetRandomNumberLocalTest.s.sol:OPGetRandomNumberLocalTestScript --fork-url {L2_RPC} --broadcast --priority-gas-price {L2_TRANSACTION_PRIORITY_GAS_PRICE}"
    if LOOTCHAIN_DEPLOYMENT:
        cmd = cmd + " --slow --legacy"
    cprint(cmd)
    run_command(
        [cmd],
        env={
            "OP_ADAPTER_ADDRESS": l2_addresses["Adapter"],
        },
        cwd=CONTRACTS_DIR,
        capture_output=HIDE_OUTPUT,
        shell=True,
    )
    l2_cur_randomness = get_last_randomness(l2_addresses["Adapter"], L2_RPC)

    # 3. Check if randomness is updated

    print("Waiting for randomness to be updated...")
    cmd = f'cast call {l2_addresses["Adapter"]} "getLastRandomness()(uint256)" --rpc-url {L2_RPC}'
    if LOOTCHAIN_DEPLOYMENT:
        cmd = cmd + " --slow --legacy"
    cprint(cmd)
    l2_cur_randomness = wait_command(
        [cmd],
        wait_time=15,
        max_attempts=20,
        fail_value=l2_prev_randomness,
        shell=True,
    )
    print(f"\nOld L2 randomness: {l2_prev_randomness}")
    print(f"New L2 randomness: {l2_cur_randomness}")
    print("L2 Requested Randomness succesfully!\n")


def print_addresses():
    l1_addresses = get_l1_addresses()
    l2_addresses = get_l2_addresses()

    addresses = {}
    addresses["L1 Addresses"] = l1_addresses
    addresses["L2 Addresses"] = l2_addresses

    with open(ADDRESSES_JSON_PATH, "w") as f:
        json.dump(addresses, f, indent=4)


def deploy_controller_relayer():
    # use forge to manually create new controller relayer on L1 with controller as the constructor argument
    print("Deploying controller relayer...")

    cmd = f"forge create src/ControllerRelayer.sol:ControllerRelayer --constructor-args {EXISTING_L1_CONTROLLER_ADDRESS} --rpc-url {L1_RPC} --private-key {ADMIN_PRIVATE_KEY}"
    cprint(cmd.replace(ADMIN_PRIVATE_KEY, "***"))
    output = run_command(
        [cmd],
        shell=True,
        cwd=CONTRACTS_DIR,
        capture_output=True,
    ).stdout.strip()

    output_str = output.decode("utf-8")
    lines = output_str.split("\n")
    deployed_to_line = [line for line in lines if "Deployed to:" in line][0]
    controller_relayer_address = deployed_to_line.split("Deployed to:")[1].strip()
    for line in lines:
        print(line)
    print(
        f"\nNew L1 ControllerRelayer deployed to: {controller_relayer_address} \nAddress has been saved to .env (EXISTING_L1_CONTROLLER_RELAYER)"
    )

    # update .env file with new controller relayer address
    set_key(ENV_PATH, "EXISTING_L1_CONTROLLER_RELAYER", controller_relayer_address)


def relay_groups(controller_oracle_address):
    # Use cast send to create a new controller relayer contract
    # determine number of available groups by calling getValidGroupIndices()
    print("Getting available group indices...")
    cmd = f'cast call {EXISTING_L1_CONTROLLER_ADDRESS} "getValidGroupIndices()(uint256[])" --rpc-url {L1_RPC}'
    cprint(cmd)
    bytes_literal = run_command(
        [cmd],
        shell=True,
        capture_output=True,
    ).stdout.strip()
    str_literal = bytes_literal.decode("UTF-8")
    group_indices = ast.literal_eval(str_literal)

    print("Available groups indices:")
    print(group_indices)

    #  call relayGroup(chainid, groupindex) for each group index
    print("Relaying groups from L1 to L2...")
    for group_index in group_indices:
        cmd = f'cast send {EXISTING_L1_CONTROLLER_RELAYER} "relayGroup(uint256,uint256)" {L2_CHAIN_ID} {group_index} --rpc-url {L1_RPC} --private-key {ADMIN_PRIVATE_KEY} --gas-limit 3000000'
        cprint(cmd.replace(ADMIN_PRIVATE_KEY, "***"))
        run_command(
            [cmd],
            shell=True,
        )

        print(f"Waiting for group index:{group_index} to relay from L1 to L2:")

        non_relayed_group = "0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"

        cmd = f'cast call {controller_oracle_address} "getGroup(uint256)" {group_index} --rpc-url {L2_RPC}'
        if LOOTCHAIN_DEPLOYMENT:
            cmd = cmd + " --slow --legacy"
        cprint(cmd)

        l2_group_info = wait_command(
            [cmd],
            wait_time=15,
            max_attempts=200,
            fail_value=non_relayed_group,
            shell=True,
        )

        if l2_group_info:
            print(f"Group {group_index} relayed from L1 to L2!")
            print(l2_group_info)


def update_group(controller_address, controller_oracle_address, group_index):
    """
    Get group info from L1 and update group on L2 using updateGroup.s.sol solidity script
    """

    # Compute the admin wallet address
    cmd = f"cast wallet address --private-key {ADMIN_PRIVATE_KEY}"
    cprint(cmd.replace(ADMIN_PRIVATE_KEY, "***"))
    admin_wallet_address = (
        run_command(
            [cmd],
            shell=True,
            capture_output=True,
        )
        .stdout.strip()
        .decode("utf-8")
    )

    print(f"Admin wallet address: {admin_wallet_address}")

    # construct the forge script
    cmd = f"forge script script/updateGroup.s.sol:GetGroupFromL1AndUpdateL2Script --broadcast --priority-gas-price {L1_TRANSACTION_PRIORITY_GAS_PRICE}"

    cprint(cmd)

    run_command(
        [cmd],
        env={
            "ADMIN_PRIVATE_KEY": ADMIN_PRIVATE_KEY,
            "ADMIN_ADDRESS": admin_wallet_address,
            "CONTROLLER_ADDRESS": controller_address,
            "OP_CONTROLLER_ORACLE_ADDRESS": controller_oracle_address,
            "L1_RPC": L1_RPC,
            "OP_RPC": L2_RPC,
        },
        cwd=CONTRACTS_DIR,
        shell=True,
    )


def main():
    # # Main deployment script
    deploy_contracts()
    deploy_nodes()
    test_request_randomness()

    ## For L2 only deployments, use the following prior to the first deployment.
    # deploy_controller_relayer()

    ## Manually call relayGroups(L2controllerOracleAddress)
    # relay_groups("0x901105C43C7f0e421b33c9D1DaA25f54076F6563")

    ## Get public/private key info from node mnemonic
    # print_node_key_info()

    ## Manually Update Group from L1 to L2
    # update_group(
    #     "0x647c919280A1cE898cBf8BD72c8a912165B4f70a",  # controller_address
    #     "0x6789dD361406E3DFC3a52BAfFD4C05958d25deDe",  # controller_oracle_address
    #     0,  # group_index
    # )


if __name__ == "__main__":
    main()
