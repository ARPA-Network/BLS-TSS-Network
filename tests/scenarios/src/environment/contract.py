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

def contract_function_transact(contract, function_name, *args):
    """
    contract is an instance of a web3 contract
    function_name is a string representing the name of the function to execute
    args is a variable-length argument list representing the parameters for the function
    
    Get the function object from the contract instance
    """
    function = getattr(contract.functions, function_name)
    max_fee = get_average_gas_price('http://127.0.0.1:8545')
    # Call the function with the provided parameters
    gas_estimate = function(*args).estimate_gas()
    result = function(*args).transact({'gasPrice': max_fee, 'gas': int(gas_estimate * 1.2)})
    return result

def get_average_gas_price(url):
    """
    This function calculates the average gas price for the last 10 blocks.
    It connects to an Ethereum node at the given URL, fetches the gas used 
    and gas limit for each of the last 10 blocks, calculates the gas price for
    each block, and returns the average gas price.
    
    Parameters:
    url (str): The HTTP provider address of the Ethereum node

    Returns:
    int: The average gas price for the last 10 blocks
    """

   # Connect to your Ethereum node
    w3 = Web3(Web3.HTTPProvider(url))

    # Get the latest block number
    latest_block_number = w3.eth.block_number

    # Initialize a list to store the gas prices
    gas_prices = []
    if url == 'http://localhost:8545':
        try:
            # Loop through the latest 10 blocks
            for i in range(latest_block_number - 9, latest_block_number + 1):
                # Get the block
                block = w3.eth.get_block(i, full_transactions=True)

                # Add the gas price of each transaction to the list
                if block is not None:
                    for tx in block['transactions']:
                        gas_prices.append(tx['gasPrice'])
                # Calculate the average gas price
        except Exception as exception:
            print(f"Error: {str(exception)}")
    if len(gas_prices) == 0:
        return w3.eth.gas_price
    average_gas_price = sum(gas_prices) / len(gas_prices)
    return int(average_gas_price * 1.2)


def exec_script(script_name, url='http://localhost:8545'):
    """
    Executes a script from the "contracts/script" directory.
    """
    max_fee = get_average_gas_price(url)
    os.chdir("contracts")
    cmd = ("forge script script/" + script_name
        + " --fork-url " + url +  " --with-gas-price " + str(max_fee) + " --broadcast --slow")
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

    for transaction in transactions:
        if transaction.get('transactionType') == 'CREATE':
            contract_address = transaction.get('contractAddress')
            contract_name = transaction.get('contractName')
            if contract_address is not None and contract_name is not None:
                # create a mapping between the contract name and its address
                contract_addresses[contract_name] = contract_address
            if contract_name is None:
                contract_addresses['default'] = contract_address

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
