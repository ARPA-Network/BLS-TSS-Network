from dotenv import load_dotenv
import json
import os
import subprocess
import sys
from web3 import Web3

def get_abi(file_name):
    #read abi from json file
    with open(file_name, 'r') as f:
        abi = json.load(f)
        abi = abi["abi"]
    return abi

def get_contract(file_name, address):
    #get the contract object
    abi = get_abi(file_name)
    w3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))
    contract = w3.eth.contract(
        address = address,
        abi = abi
    )
    return contract

def to_checksum_address(lower_case_address):
    #Web3.py only accepts checksum addresses

    return lower_case_address

def contract_function_call(contract, function_name, *args):
    # contract is an instance of a web3 contract
    # function_name is a string representing the name of the function to execute
    # args is a variable-length argument list representing the parameters for the function
    # Get the function object from the contract instance
    function = getattr(contract.functions, function_name)
    # Call the function with the provided parameters
    result = function(*args).call()
    return result

def contract_function_transact(contract, function_name, *args):
    # contract is an instance of a web3 contract
    # function_name is a string representing the name of the function to execute
    # args is a variable-length argument list representing the parameters for the function
    
    # Get the function object from the contract instance
    function = getattr(contract.functions, function_name)
    
    # Call the function with the provided parameters
    result = function(*args).transact()
    
    return result

def exec_script(script_name):
    os.chdir("contracts")
    cmd = "forge script script/" + script_name + " --fork-url http://localhost:8545 --broadcast"
    os.system(cmd)
    os.chdir("..")

def get_value_from_env(name):
    load_dotenv("contracts/.env")
    value = os.environ.get(name)
    return value

def set_value_to_env(name, value):
    load_dotenv("contracts/.env")
    os.environ[name] = value
    value = os.environ.get(name)
    print("Set value to env: ", name, value)
    os.system("source contracts/.env")

def deploy_controller():
    cmd = ["forge", "script", "script/DeployControllerLocalTest.s.sol:DeployControllerTestScript",
           "--fork-url", "http://localhost:8545", "--broadcast"]
    p = subprocess.Popen([sys.executable, '-c', 'import pty, sys; pty.spawn(sys.argv[1:])',
                         *cmd], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, cwd="contracts")
    out = p.communicate(b"y")[0]
    print(out.decode('utf-8'))
    p.wait()

deploy_controller()




# def config_proxy(proxy):
#     w3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))
#     load_dotenv("contracts/.env")
#     node_staking_amount = os.environ.get("NODE_STAKING_AMOUNT")
#     disqualified_node_penalty_amount = os.environ.get("DISQUALIFIED_NODE_PENALTY_AMOUNT")
#     default_number_of_committers = os.environ.get("DEFAULT_NUMBER_OF_COMMITTERS")
#     default_dkg_phase_duration = os.environ.get("DEFAULT_DKG_PHASE_DURATION")
#     group_max_capacity = os.environ.get("GROUP_MAX_CAPACITY")
#     ideal_number_of_groups = os.environ.get("IDEAL_NUMBER_OF_GROUPS")
#     pending_block_after_quit = os.environ.get("PENDING_BLOCK_AFTER_QUIT")
#     dkg_post_process_reward = os.environ.get("DKG_POST_PROCESS_REWARD")

#     minimum_request_confirmations = os.environ.get("MINIMUM_REQUEST_CONFIRMATIONS")
#     max_gas_limit = os.environ.get("MAX_GAS_LIMIT")
#     staleness_seconds = os.environ.get("STALENESS_SECONDS")
#     gas_after_payment_calculation = os.environ.get("GAS_AFTER_PAYMENT_CALCULATION")
#     gas_except_callback = os.environ.get("GAS_EXCEPT_CALLBACK")
#     fallback_wei_per_unit_arpa = os.environ.get("FALLBACK_WEI_PER_UNIT_ARPA")
#     signature_task_exclusive_window = os.environ.get("SIGNATURE_TASK_EXCLUSIVE_WINDOW")
#     reward_per_signature = os.environ.get("REWARD_PER_SIGNATURE")
#     committer_reward_per_signature = os.environ.get("COMMITTER_REWARD_PER_SIGNATURE")

#     fulfillment_flat_fee_arpa_ppm_tier1 = os.environ.get("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER1")
#     fulfillment_flat_fee_arpa_ppm_tier2 = os.environ.get("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER2")
#     fulfillment_flat_fee_arpa_ppm_tier3 = os.environ.get("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER3")
#     fulfillment_flat_fee_arpa_ppm_tier4 = os.environ.get("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER4")
#     fulfillment_flat_fee_arpa_ppm_tier5 = os.environ.get("FULFILLMENT_FLAT_FEE_ARPA_PPM_TIER5")
#     reqs_for_tier2 = os.environ.get("REQS_FOR_TIER2")
#     reqs_for_tier3 = os.environ.get("REQS_FOR_TIER3")
#     reqs_for_tier4 = os.environ.get("REQS_FOR_TIER4")
#     reqs_for_tier5 = os.environ.get("REQS_FOR_TIER5")


#     ret = proxy.functions.setControllerConfig(
#         int(node_staking_amount, 10),
#         int(disqualified_node_penalty_amount, 10),
#         int(default_number_of_committers, 10),
#         int(default_dkg_phase_duration, 10),
#         int(group_max_capacity, 10),
#         int(ideal_number_of_groups, 10),
#         int(pending_block_after_quit, 10),
#         int(dkg_post_process_reward, 10)
#     ).transact()
#     time.sleep(1)
#     tx_receipt = w3.eth.getTransactionReceipt(ret)
#     print('Config proxy controller config: ', tx_receipt)

#     fee_config = {
#         'fulfillmentFlatFeeArpaPPMTier1': int(fulfillment_flat_fee_arpa_ppm_tier1, 10),
#         'fulfillmentFlatFeeArpaPPMTier2': int(fulfillment_flat_fee_arpa_ppm_tier2, 10),
#         'fulfillmentFlatFeeArpaPPMTier3': int(fulfillment_flat_fee_arpa_ppm_tier3, 10),
#         'fulfillmentFlatFeeArpaPPMTier4': int(fulfillment_flat_fee_arpa_ppm_tier4, 10),
#         'fulfillmentFlatFeeArpaPPMTier5': int(fulfillment_flat_fee_arpa_ppm_tier5, 10),
#         'reqsForTier2': int(reqs_for_tier2, 10),
#         'reqsForTier3': int(reqs_for_tier3, 10),
#         'reqsForTier4': int(reqs_for_tier4, 10),
#         'reqsForTier5': int(reqs_for_tier5, 10)
#     }
#     #fee_config_tuple = tuple(fee_config.items())
#     ret = proxy.functions.setAdapterConfig(
#         int(minimum_request_confirmations, 10),
#         int(max_gas_limit, 10),
#         int(staleness_seconds, 10),
#         int(gas_after_payment_calculation, 10),
#         int(gas_except_callback, 10),
#         int(fallback_wei_per_unit_arpa, 10),
#         int(signature_task_exclusive_window, 10),
#         int(reward_per_signature, 10),
#         int(committer_reward_per_signature, 10),
#         fee_config
#     ).transact()
#     time.sleep(1)
#     tx_receipt = w3.eth.getTransactionReceipt(ret)
#     print('Config proxy adapter config: ', tx_receipt)
