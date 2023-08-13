import os
import subprocess
import json
from pprint import pprint
from dotenv import load_dotenv, set_key
import ruamel.yaml


def run_command(args, check=True, shell=False, cwd=None, env=None):
    env = env if env else {}
    return subprocess.run(
        args, check=check, shell=shell, env={**os.environ, **env}, cwd=cwd
    )


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
    ###### Contract Deployment #######
    ##################################

    # ###############################
    # # 1. Copy .env.example to .env, and load .env file for editing
    # run_command(["cp", env_example_path, env_path])
    # load_dotenv(dotenv_path=env_path)

    # ###############################
    # # 2. Deploy L2 OPControllerOracleLocalTest contracts (ControllerOracle, Adapter, Arpa)
    # # # forge script script/OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript --fork-url http://localhost:9545 --broadcast
    # run_command(
    #     [
    #         "forge",
    #         "script",
    #         "script/OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript",
    #         "--fork-url",
    #         "http://localhost:9545",
    #         "--broadcast",
    #     ],
    #     env={},
    #     cwd=contracts_dir,
    # )
    # # get L2 contract addresses from broadcast and update .env file
    # l2_addresses = get_addresses_from_json(op_controller_oracle_broadcast_path)
    # set_key(env_path, "OP_ADAPTER_ADDRESS", l2_addresses["Adapter"])
    # set_key(env_path, "OP_ARPA_ADDRESS", l2_addresses["Arpa"])
    # set_key(env_path, "OP_CONTROLLER_ORACLE_ADDRESS", l2_addresses["ControllerOracle"])

    # ###############################
    # # 3. Deploy L1 ControllerLocalTest contracts
    # #     (Controller, Controller Relayer, OPChainMessenger, Adapter, Arpa, Staking)

    # # forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript --fork-url http://localhost:8545 --broadcast
    # run_command(
    #     [
    #         "forge",
    #         "script",
    #         "script/ControllerLocalTest.s.sol:ControllerLocalTestScript",
    #         "--fork-url",
    #         "http://localhost:8545",
    #         "--broadcast",
    #     ],
    #     env={},
    #     cwd=contracts_dir,
    # )

    # # get L1 contract addresses from broadcast and update .env file
    # l1_addresses = get_addresses_from_json(controller_local_test_broadcast_path)
    # set_key(env_path, "ARPA_ADDRESS", l1_addresses["Arpa"])
    # set_key(env_path, "STAKING_ADDRESS", l1_addresses["Staking"])
    # set_key(env_path, "CONTROLLER_ADDRESS", l1_addresses["Controller"])
    # set_key(env_path, "ADAPTER_ADDRESS", l1_addresses["Adapter"])

    # # reload new .env file
    # load_dotenv(dotenv_path=env_path)
    # # TODO: This is not working... had to fill in env manually below.

    # ###############################
    # # 4. deploy remaining contracts (Controller Oracle Init, StakeNodeLocalTest)

    # # forge script script/OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript --fork-url http://localhost:9545 --broadcast
    # run_command(
    #     [
    #         "forge",
    #         "script",
    #         "script/OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript",
    #         "--fork-url",
    #         "http://localhost:9545",
    #         "--broadcast",
    #     ],
    #     env={
    #         "OP_ADAPTER_ADDRESS": l2_addresses["Adapter"],
    #         "OP_ARPA_ADDRESS": l2_addresses["Arpa"],
    #         "OP_CONTROLLER_ORACLE_ADDRESS": l2_addresses["ControllerOracle"],
    #         "ARPA_ADDRESS": l1_addresses["Arpa"],
    #         "STAKING_ADDRESS": l1_addresses["Staking"],
    #         "CONTROLLER_ADDRESS": l1_addresses["Controller"],
    #         "ADAPTER_ADDRESS": l1_addresses["Adapter"],
    #     },
    #     cwd=contracts_dir,
    # )

    # # forge script script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript --fork-url http://localhost:8545 --broadcast
    # run_command(
    #     [
    #         "forge",
    #         "script",
    #         "script/StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript",
    #         "--fork-url",
    #         "http://localhost:8545",
    #         "--broadcast",
    #         "-g",
    #         "150",
    #     ],
    #     env={
    #         "OP_ADAPTER_ADDRESS": l2_addresses["Adapter"],
    #         "OP_ARPA_ADDRESS": l2_addresses["Arpa"],
    #         "OP_CONTROLLER_ORACLE_ADDRESS": l2_addresses["ControllerOracle"],
    #         "ARPA_ADDRESS": l1_addresses["Arpa"],
    #         "STAKING_ADDRESS": l1_addresses["Staking"],
    #         "CONTROLLER_ADDRESS": l1_addresses["Controller"],
    #         "ADAPTER_ADDRESS": l1_addresses["Adapter"],
    #     },
    #     cwd=contracts_dir,
    # )

    ######################################
    ###### ARPA Network Deployment #######
    ######################################

    # update config.yml files with correect L1 controller and adapter addresses
    l1_addresses = get_addresses_from_json(controller_local_test_broadcast_path)

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

    # start notes
    # run_command(["docker", "compose", "up", "-d"], cwd=script_dir)

    # test to make sure things are good


if __name__ == "__main__":
    main()
