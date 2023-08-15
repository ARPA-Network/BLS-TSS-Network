import json
import os
import subprocess
import sys
import time
from pprint import pprint

import ruamel.yaml
from dotenv import load_dotenv, set_key


def get_addresses_from_json(path: str) -> dict:
    """
    Given a path to a json file, return a dictionary of contract names to addresses
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
            if contract_name and contract_address:
                contracts_dict[contract_name] = contract_address

    return contracts_dict


def run_command(
    cmd: list,
    check=True,
    shell=False,
    cwd=None,
    env=None,
    capture_output=False,
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
    wait_time=10,
    max_attempts=12,
    fail_value=None,
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
        fail_value (str): value that indicates an unsuccessful command even if it executes without raising an exception

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
            capture_output=True,
            text=True,
        )
        # If the command is successful but stdout matches fail_value increase fail_counter
        if (
            command_output.returncode == 0
            and command_output.stdout.strip() != fail_value
        ):
            print("command_output.stdout")
            print(command_output.stdout)
            print("fail_value")
            print(fail_value)
            return command_output.stdout
        else:
            print("...")
            fail_counter += 1

        # If the command fails for max_attempts consecutive times, return None
        if fail_counter >= max_attempts:
            print(
                f"Error: Command did not finish after {wait_time*max_attempts} seconds. Exiting..."
            )
            return None

        time.sleep(wait_time)


def main():
    l1_chain_id = "900"
    l2_chain_id = "901"

    # prep directories
    script_dir = os.getcwd()
    root_dir = os.path.abspath(os.path.join(script_dir, os.pardir))
    arpa_node_dir = os.path.join(script_dir, "arpa-node")
    contracts_dir = os.path.join(root_dir, "contracts")
    env_example_path = os.path.join(contracts_dir, ".env.example")
    env_path = os.path.join(contracts_dir, ".env")
    op_controller_oracle_broadcast_path = os.path.join(
        contracts_dir,
        "broadcast",
        "OPControllerOracleLocalTest.s.sol",
        l2_chain_id,
        "run-latest.json",
    )
    controller_local_test_broadcast_path = os.path.join(
        contracts_dir,
        "broadcast",
        "ControllerLocalTest.s.sol",
        l1_chain_id,
        "run-latest.json",
    )

    ##################################
    #!#### Contract Deployment #######
    ##################################

    ###############################
    # 1. Copy .env.example to .env, and load .env file for editing
    run_command(["cp", env_example_path, env_path])
    load_dotenv(dotenv_path=env_path)

    ###############################
    # 2. Deploy L2 OPControllerOracleLocalTest contracts (ControllerOracle, Adapter, Arpa)
    # # forge script script/OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript --fork-url http://localhost:9545 --broadcast
    run_command(
        [
            "forge",
            "script",
            "script/OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript",
            "--fork-url",
            "http://localhost:9545",
            "--broadcast",
        ],
        env={},
        cwd=contracts_dir,
    )
    # get L2 contract addresses from broadcast and update .env file
    l2_addresses = get_addresses_from_json(op_controller_oracle_broadcast_path)
    set_key(env_path, "OP_ADAPTER_ADDRESS", l2_addresses["ERC1967Proxy"])
    set_key(env_path, "OP_ARPA_ADDRESS", l2_addresses["Arpa"])
    set_key(env_path, "OP_CONTROLLER_ORACLE_ADDRESS", l2_addresses["ControllerOracle"])

    ###############################
    # 3. Deploy L1 ControllerLocalTest contracts
    #     (Controller, Controller Relayer, OPChainMessenger, Adapter, Arpa, Staking)

    # forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://localhost:8545 --broadcast
    run_command(
        [
            "forge",
            "script",
            "script/ControllerLocalTest.s.sol:ControllerLocalTestScript",
            "--fork-url",
            "http://localhost:8545",
            "--broadcast",
        ],
        env={},
        cwd=contracts_dir,
    )

    # get L1 contract addresses from broadcast and update .env file
    l1_addresses = get_addresses_from_json(controller_local_test_broadcast_path)
    set_key(env_path, "ARPA_ADDRESS", l1_addresses["Arpa"])
    set_key(env_path, "STAKING_ADDRESS", l1_addresses["Staking"])
    set_key(env_path, "CONTROLLER_ADDRESS", l1_addresses["Controller"])
    set_key(env_path, "ADAPTER_ADDRESS", l1_addresses["ERC1967Proxy"])

    # reload new .env file
    load_dotenv(dotenv_path=env_path)
    # TODO: This is not working... had to fill in env manually below.

    ###############################
    # 4. deploy remaining contracts (Controller Oracle Init, StakeNodeLocalTest)

    # forge script script/OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript --fork-url http://localhost:9545 --broadcast
    run_command(
        [
            "forge",
            "script",
            "script/OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript",
            "--fork-url",
            "http://localhost:9545",
            "--broadcast",
        ],
        env={
            "OP_ADAPTER_ADDRESS": l2_addresses["ERC1967Proxy"],
            "OP_ARPA_ADDRESS": l2_addresses["Arpa"],
            "OP_CONTROLLER_ORACLE_ADDRESS": l2_addresses["ControllerOracle"],
        },
        cwd=contracts_dir,
    )

    # forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://localhost:8545 --broadcast
    run_command(
        [
            "forge",
            "script",
            "script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript",
            "--fork-url",
            "http://localhost:8545",
            "--broadcast",
            "-g",
            "150",
            "--slow",  # important
        ],
        env={
            "ARPA_ADDRESS": l1_addresses["Arpa"],
            "STAKING_ADDRESS": l1_addresses["Staking"],
            "ADAPTER_ADDRESS": l1_addresses["ERC1967Proxy"],
        },
        cwd=contracts_dir,
    )

    ######################################
    #!#### ARPA Network Deployment #######
    ######################################

    # update config.yml files with correect L1 controller and adapter addresses
    l1_addresses = get_addresses_from_json(controller_local_test_broadcast_path)
    # l2_addresses = get_addresses_from_json(op_controller_oracle_broadcast_path)

    config_files = ["config_1.yml", "config_2.yml", "config_3.yml"]
    yaml = ruamel.yaml.YAML()
    yaml.preserve_quotes = True  # preserves quotes
    yaml.indent(sequence=4, offset=2)  # set indentation

    for file in config_files:
        file_path = os.path.join(arpa_node_dir, file)
        with open(file_path, "r") as f:
            data = yaml.load(f)
        data["adapter_address"] = l1_addresses["Arpa"]
        data["controller_address"] = l1_addresses["Controller"]
        with open(file_path, "w") as f:
            yaml.dump(data, f)

    # start randcast nodes
    print("Starting randcast nodes...")
    run_command(["docker", "compose", "up", "-d"], cwd=script_dir)

    # wait for succesful grouping (fail after 1m without grouping)
    print("Waiting for nodes to group...")
    cmd = [
        "docker",
        "exec",
        "-it",
        "node1",
        "sh",
        "-c",
        "cat /var/log/randcast_node_client.log | grep 'available'",
    ]

    nodes_grouped = wait_command(cmd, wait_time=10, max_attempts=12)

    if nodes_grouped:
        print("Nodes grouped succesfully!")
        print("Output:\n", nodes_grouped)
    else:
        print("Nodes failed to group!")
        run_command(
            [
                "docker",
                "exec",
                "-it",
                "node1",
                "sh",
                "-c",
                "cat /var/log/randcast_node_client.log | tail",
            ]
        )
        sys.exit(1)

    # ############################################
    # #!#### L1 Request Randomness Testing #######
    # ############################################

    l1_addresses = get_addresses_from_json(controller_local_test_broadcast_path)
    l2_addresses = get_addresses_from_json(op_controller_oracle_broadcast_path)
    # pprint(l1_addresses)
    # pprint(l2_addresses)

    # deploy user contract and request randomness
    # forge script /usr/src/app/script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url $ETH_RPC_URL --broadcast
    print("Deploying L1 user contract and requesting randomness...")
    run_command(
        [
            "forge",
            "script",
            "script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript",
            "--fork-url",
            "http://localhost:8545",
            "--broadcast",
        ],
        env={
            "ADAPTER_ADDRESS": l1_addresses["ERC1967Proxy"],
        },
        cwd=contracts_dir,
    )

    # confirm randomness request suceeded with the adapter
    # cast call 0xa513e6e4b8f2a923d98304ec87f64353c4d5c853 "getLastRandomness()(uint256)" # should not show 0

    print("Check if randomness request succeeded...")
    cmd = [
        "cast",
        "call",
        l1_addresses["ERC1967Proxy"],
        "getLastRandomness()(uint256)",
    ]
    adapter_randomness_result = wait_command(
        cmd, cwd=contracts_dir, wait_time=10, max_attempts=12, fail_value="0"
    )
    if adapter_randomness_result:
        print("Adapter randomness request succeeded!")
        print("Output:\n", adapter_randomness_result)
    else:
        print("Adapter randomness request failed!")
        sys.exit(1)

    # # cast call 0x712516e61C8B383dF4A63CFe83d7701Bce54B03e "lastRandomnessResult()(uint256)" # should match above

    # ############################################
    # #!#### L2 Request Randomness Testing #######
    # ############################################


if __name__ == "__main__":
    main()
