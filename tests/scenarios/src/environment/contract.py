"""
This module contains necessary packages to interact with web3 and blockchain 
"""
import json
import os

from web3 import Web3, Account

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

def contract_function_transact(contract, function_name, max_price=1000, *args):
    """
    contract is an instance of a web3 contract
    function_name is a string representing the name of the function to execute
    args is a variable-length argument list representing the parameters for the function
    
    Get the function object from the contract instance
    """
    function = getattr(contract.functions, function_name)
    # Call the function with the provided parameters
    #gas_estimate = function(*args).estimate_gas()
    print(max_price)
    result = function(*args).transact()
    return result


def exec_script(script_name, url='http://localhost:8545', max_price=10000):
    """
    Executes a script from the "contracts/script" directory.
    """
    print(max_price)
    if len(str(url)) == 0:
        url = 'http://localhost:8545'
    os.chdir("contracts")
    cmd = ("forge script script/" + script_name
        + " --fork-url " + url +  " --gas-price " + str(max_price) + " --broadcast --slow")
    os.system(cmd)
    os.chdir("..")

def get_contract_address_from_file(file_name):
    """
    Get contract address from receipt file.
    file_name: the receipt file name.
    returns: dictionary mapping contract names to their addresses
    """
    with open(file_name, 'r', encoding='UTF-8') as file:
        data = json.load(file)

    transactions = data.get('transactions')

    # initialize an empty dictionary to hold the mapping
    contract_addresses = {}
    count = 0
    for transaction in transactions:
        if transaction.get('transactionType') == 'CREATE':
            contract_address = transaction.get('contractAddress')
            contract_address = to_checksum_address(contract_address)
            contract_name = transaction.get('contractName')
            if contract_address is not None and contract_name is not None:
                # create a mapping between the contract name and its address
                if (contract_name == "ERC1967Proxy"):
                    contract_name = "ERC1967Proxy" + str(count)
                    count += 1
                contract_addresses[contract_name] = contract_address
            if contract_name is None:
                contract_addresses['default'] = contract_address

    return contract_addresses

def get_contract_address_from_json(path):
    """
    Fetches all contract addresses from a JSON file and returns them in a single dictionary.
    """
    
    # Read and parse the JSON file
    try:
        with open(path, 'r') as file:
            data = json.load(file)
            return data
    except FileNotFoundError:
        print(f"File {path} not found.")
        return None


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


def call_cancel_overtime_request_by_event(contract, event):
    """
    Call cancel overtime request by event.
    """
    web3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))
    private_key = os.environ.get('USER_PRIVATE_KEY')
    if not private_key:
        print("Private key is not found!")
        return None
    try:
        contract_function = contract.functions.cancelOvertimeRequest(
            event['args']['requestId'],
            (
                event['args']['subId'],event['args']['groupIndex'],
                event['args']['requestType'],event['args']['params'],
                event['args']['sender'],event['args']['seed'],
                event['args']['requestConfirmations'],
                event['args']['callbackGasLimit'],
                event['args']['callbackMaxGasPrice'],
                event['blockNumber']
            )
        )
    except Exception as exception:
        print(f"Contract function error: {str(exception)}")
        return None
    transaction = contract_function.build_transaction({
        'chainId': 31337,
        'gas': 10000000,
        'nonce': web3.eth.get_transaction_count(Account.from_key(private_key).address),
    })

    signed_transaction = web3.eth.account.sign_transaction(transaction, private_key)

    result = web3.eth.send_raw_transaction(signed_transaction.rawTransaction)

    return result
