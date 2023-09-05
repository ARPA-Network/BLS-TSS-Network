"""
This module contains necessary packages to interact with web3 and blockchain 
"""
import json
import os

from web3 import Web3

def get_abi(file_name):
    """
    Read an ABI (Application Binary Interface) string from a JSON file.
    """
    with open(file_name, 'r', encoding='utf-8') as file:
        abi = json.load(file)
        abi = abi["abi"]
    return abi

def get_contract(file_name, address):
    """
    Get the contract object.
    """
    abi = get_abi(file_name)
    web3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))
    contract = web3.eth.contract(
        address = address,
        abi = abi
    )
    return contract

def to_checksum_address(lower_case_address):
    """
    Web3.py only accepts checksum addresses
    To be removed, new version of web3.py will accept lower case addresses
    """
    return Web3.to_checksum_address(lower_case_address)

def contract_function_call(contract, function_name, *args):
    """
    contract is an instance of a web3 contract
    function_name is a string representing the name of the function to execute
    args is a variable-length argument list representing the parameters for the function
    Get the function object from the contract instance
    """
    function = getattr(contract.functions, function_name)
    # Call the function with the provided parameters
    result = function(*args).call()
    return result

def contract_function_transact(contract, function_name, *args):
    """
    contract is an instance of a web3 contract
    function_name is a string representing the name of the function to execute
    args is a variable-length argument list representing the parameters for the function
    
    Get the function object from the contract instance
    """
    function = getattr(contract.functions, function_name)
    # Call the function with the provided parameters
    result = function(*args).transact()
    return result

def exec_script(script_name):
    """
    Executes a script from the "contracts/script" directory.
    """
    os.chdir("contracts")
    cmd = ("forge script script/" + script_name
        + " --fork-url http://localhost:8545 --broadcast --slow")
    os.system(cmd)
    os.chdir("..")

def get_contract_address_from_file(file_name):
    """
    Get contract address from receipt file.
    file_name: the receipt file name.
    """
    with open(file_name, 'r', encoding='UTF-8') as file:
        data = json.load(file)
    transactions = data.get('transactions')
    contract_addresses = []
    for transaction in transactions:
        contract_address = transaction.get('contractAddress')
        if contract_address is not None:
            contract_addresses.append(contract_address)
    return contract_addresses

def get_event(contract, event_name, from_block=0):
    """
    Get event from contract.
    """
    event = getattr(contract.events, event_name)
    event_filter = event.create_filter(fromBlock=from_block)
    log = event_filter.get_new_entries()
    if len(log) == 0:
        return None
    print(log[0])
    return log[0]

def get_latest_event(contract, event_name, from_block=0):
    """
    Get latest event from contract.
    """
    event = getattr(contract.events, event_name)
    event_filter = event.create_filter(fromBlock=from_block)
    log = event_filter.get_new_entries()
    if len(log) == 0:
        return None
    print(log[-1])
    return log[-1]

def get_events(contract, event_name, from_block=0):
    """
    Get events from contract.
    """
    event = getattr(contract.events, event_name)
    event_filter = event.create_filter(fromBlock=from_block)
    logs = event_filter.get_new_entries()
    print(logs)
    return logs

def events_should_contain_all_value(events, key, *value):
    """
    Check if all events contain the value of the key.
    """
    #cast value to upper case
    value = [str(v).upper() for v in value]
    for event in events:
        if str(event['args'][key]).upper() not in value:
            return False
        print(str(event['args'][key])+ " is in evnets")
    return True

def events_values_should_be(events, key, value):
    """
    Check if all events contain the value of the key.
    """
    for event in events:
        if str(event['args'][key]) != str(value):
            return False
    return True

def clac_reward(amount, reward_ratio, ):
    """
    Calculate the reward amount.
    """
    return amount * reward_ratio / 100
